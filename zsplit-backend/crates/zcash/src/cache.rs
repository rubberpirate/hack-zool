use async_trait::async_trait;
use std::{collections::BTreeMap, io, sync::Mutex};
use zcash_client_backend::{
    data_api::{
        chain::{BlockCache, BlockSource, error},
        scanning::ScanRange,
    },
    proto::compact_formats::CompactBlock,
};
use zcash_protocol::consensus::BlockHeight;

/// Compact blocks are disposable: the SDK's native WalletDb persists scanned state.
/// Sync downloads small batches, avoiding a duplicate unbounded on-disk chain cache.
#[derive(Default)]
pub(crate) struct MemoryCache(Mutex<BTreeMap<u32, CompactBlock>>);
impl BlockSource for MemoryCache {
    type Error = io::Error;
    fn with_blocks<F, W>(
        &self,
        from: Option<BlockHeight>,
        limit: Option<usize>,
        mut f: F,
    ) -> Result<(), error::Error<W, Self::Error>>
    where
        F: FnMut(CompactBlock) -> Result<(), error::Error<W, Self::Error>>,
    {
        let blocks = self
            .0
            .lock()
            .map_err(|_| error::Error::BlockSource(io::Error::other("cache poisoned")))?;
        for (_, b) in blocks
            .range(from.map(u32::from).unwrap_or(0)..)
            .take(limit.unwrap_or(usize::MAX))
        {
            f(b.clone())?;
        }
        Ok(())
    }
}
#[async_trait]
impl BlockCache for MemoryCache {
    fn get_tip_height(&self, range: Option<&ScanRange>) -> Result<Option<BlockHeight>, io::Error> {
        Ok(self
            .0
            .lock()
            .map_err(|_| io::Error::other("cache poisoned"))?
            .keys()
            .rev()
            .find(|h| range.is_none_or(|r| r.block_range().contains(&BlockHeight::from_u32(**h))))
            .copied()
            .map(BlockHeight::from_u32))
    }
    async fn read(&self, range: &ScanRange) -> Result<Vec<CompactBlock>, io::Error> {
        let blocks = self
            .0
            .lock()
            .map_err(|_| io::Error::other("cache poisoned"))?;
        let mut result = Vec::new();
        for height in u32::from(range.block_range().start)..u32::from(range.block_range().end) {
            match blocks.get(&height) {
                Some(b) => result.push(b.clone()),
                None => break,
            }
        }
        Ok(result)
    }
    async fn insert(&self, blocks: Vec<CompactBlock>) -> Result<(), io::Error> {
        let mut cache = self
            .0
            .lock()
            .map_err(|_| io::Error::other("cache poisoned"))?;
        // SDK sync 0.25 downloads one range, scans it, then downloads the next.
        // Evict the preceding range even though SDK defers its delete future.
        cache.clear();
        for b in blocks {
            let h = u32::try_from(b.height).map_err(|_| io::Error::other("height out of range"))?;
            cache.insert(h, b);
        }
        Ok(())
    }
    async fn delete(&self, range: ScanRange) -> Result<(), io::Error> {
        self.0
            .lock()
            .map_err(|_| io::Error::other("cache poisoned"))?
            .retain(|h, _| !range.block_range().contains(&BlockHeight::from_u32(*h)));
        Ok(())
    }
}
