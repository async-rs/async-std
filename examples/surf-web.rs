/* TODO: Once the next version of surf released, re-enable this example.
//! Sends an HTTP request to the Rust website.

use async_std::task;

fn main() -> Result<(), surf::Exception> {
    thread::spawn_task(async {
        let url = "https://www.rust-lang.org";
        let mut response = surf::get(url).await?;
        let body = response.body_string().await?;

        dbg!(url);
        dbg!(response.status());
        dbg!(response.version());
        dbg!(response.headers());
        dbg!(body.len());

        Ok(())
    })
}
*/

fn main() {}
