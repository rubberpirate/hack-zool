"""Deployment safety checks with isolated Git/state fixtures; no cloud or chain writes."""
import io
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
REVISION = 'a' * 40


class DeploymentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.repo = self.root / 'repo'
        (self.repo / 'deploy/gcp').mkdir(parents=True)
        for name in ('deploy.sh', 'release.sh'):
            shutil.copy(ROOT / 'deploy/gcp' / name, self.repo / 'deploy/gcp' / name)
        for name in ('Dockerfile', 'package-lock.json', 'zsplit-backend/Dockerfile',
                     'deploy/gcp/compose.yaml', 'deploy/gcp/Caddyfile'):
            p = self.repo / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text('fixture\n')
        self.git('init', '-q')
        self.git('config', 'user.email', 'test@example.invalid')
        self.git('config', 'user.name', 'Deployment test')
        self.git('add', '.')
        self.git('commit', '-qm', 'fixture')

    def tearDown(self):
        self.temp.cleanup()

    def git(self, *args):
        return subprocess.run(['git', '-C', str(self.repo), *args], check=True,
                              capture_output=True, text=True).stdout.strip()

    def check(self):
        return subprocess.run(['bash', str(self.repo / 'deploy/gcp/deploy.sh'), '--check', 'HEAD'],
                              cwd=self.repo, capture_output=True, text=True)

    def test_committed_archive_validates_without_ssh(self):
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(self.git('rev-parse', 'HEAD'), result.stdout)
        self.assertIn('No VM changes made', result.stdout)

    def test_uncommitted_secret_is_not_exported(self):
        (self.repo / '.env').write_text('SECRET=not-a-real-key\n')
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('uncommitted changes are excluded', result.stderr)

    def test_tracked_secret_is_rejected(self):
        (self.repo / '.env').write_text('SECRET=not-a-real-key\n')
        self.git('add', '.env')
        self.git('commit', '-qm', 'unsafe fixture')
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Sensitive file is tracked', result.stderr)

    def test_commit_without_deployment_files_is_rejected(self):
        self.git('rm', 'deploy/gcp/compose.yaml')
        self.git('commit', '-qm', 'incomplete fixture')
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Commit and push', result.stderr)

    def release(self, failure=''):
        base = self.root / 'vm'
        (base / 'app/deploy/gcp').mkdir(parents=True)
        (base / 'app/deploy/gcp/compose.yaml').write_text('fixture\n')
        (base / 'config').mkdir()
        config = base / 'config/production.env'
        config.write_text('DOMAIN="test.example.invalid"\nSECRET="fixture-only"\n')
        (base / 'state/zcash/wallet').mkdir(parents=True)
        (base / 'state/zcash/coordinator.seed').write_text('unspendable-test-fixture')
        (base / 'state/zcash/wallet/intents.sqlite3').write_text('journal-fixture')
        archive = self.root / 'source.tar.gz'
        with tarfile.open(archive, 'w:gz') as output:
            for name in ('deploy/gcp/compose.yaml', 'deploy/gcp/Caddyfile'):
                data = b'fixture\n'
                info = tarfile.TarInfo(name)
                info.size = len(data)
                output.addfile(info, io.BytesIO(data))
        binaries = self.root / 'bin'
        binaries.mkdir()
        docker = binaries / 'docker'
        docker.write_text('''#!/bin/bash
printf '%s\\n' "$*" >> "$DEPLOY_TEST_LOG"
if [[ " $* " == *" build api web "* && $DEPLOY_TEST_FAILURE == build ]]; then exit 7; fi
if [[ " $* " == *" ps -q api "* ]]; then echo fixture-container; fi
if [[ " $* " == *" pg_dump "* ]]; then
  if [[ $DEPLOY_TEST_FAILURE == backup ]]; then exit 8; fi
  echo fixture-database-backup
fi
''')
        docker.chmod(0o755)
        curl = binaries / 'curl'
        curl.write_text('#!/bin/bash\n[[ $DEPLOY_TEST_FAILURE != health ]]\n')
        curl.chmod(0o755)
        log = self.root / 'docker.log'
        result = subprocess.run(['bash', str(ROOT / 'deploy/gcp/release.sh'), REVISION, str(archive)],
                                env={**os.environ, 'PATH': str(binaries) + ':' + os.environ['PATH'],
                                     'ZOOL_DEPLOY_ROOT': str(base), 'DEPLOY_TEST_LOG': str(log),
                                     'DEPLOY_TEST_FAILURE': failure}, capture_output=True, text=True)
        self.assertEqual(config.read_text(), 'DOMAIN="test.example.invalid"\nSECRET="fixture-only"\n')
        self.assertEqual((base / 'state/zcash/coordinator.seed').read_text(), 'unspendable-test-fixture')
        self.assertEqual((base / 'state/zcash/wallet/intents.sqlite3').read_text(), 'journal-fixture')
        return result, base, log.read_text()

    def test_success_backs_up_and_records_commit_without_replacing_secrets(self):
        result, base, log = self.release()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((base / 'deployed-commit').read_text().strip(), REVISION)
        backup = next((base / 'backups').iterdir())
        self.assertTrue((backup / 'complete').exists())
        with tarfile.open(backup / 'zcash.tar.gz') as archive:
            self.assertEqual(archive.extractfile('zcash/coordinator.seed').read(), b'unspendable-test-fixture')
        self.assertLess(log.index('build api web'), log.index('container stop --timeout -1'))
        self.assertLess(log.index('pg_dump'), log.index('up -d --no-build'))
        self.assertNotIn('seed', log)

    def test_build_failure_leaves_running_services_alone(self):
        result, base, log = self.release('build')
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('container stop', log)
        self.assertFalse((base / 'current').exists())

    def test_backup_failure_restarts_old_api_and_never_applies_release(self):
        result, base, log = self.release('backup')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('start api', log)
        self.assertNotIn('up -d --no-build', log)
        self.assertFalse((base / 'current').exists())

    def test_health_failure_does_not_blindly_roll_back_migrations(self):
        result, base, log = self.release('health')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('No automatic rollback', result.stderr)
        self.assertNotIn('start api', log)
        self.assertFalse((base / 'current').exists())
        self.assertTrue(any((base / 'backups').glob('*/complete')))


if __name__ == '__main__':
    unittest.main()
