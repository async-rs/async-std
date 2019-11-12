use async_std::task;

fn main() -> Result<(), surf::Exception> {
    task::block_on(async {
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
