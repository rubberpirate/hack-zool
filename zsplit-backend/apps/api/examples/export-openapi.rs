use utoipa::OpenApi;
fn main() {
    println!(
        "{}",
        zsplit_api::ApiDoc::openapi()
            .to_pretty_json()
            .expect("serialize OpenAPI")
    );
}
