use async_std::prelude::*;
use async_std::io;
use async_std::task;

#[test]
fn test_send() -> io::Result<()> {
    task::block_on(async {
        fn test_send_trait<T: Send>(_: &T) {}

        let stream = futures::stream::pending::<()>();
        test_send_trait(&stream);

        let fut = stream.collect::<Vec<_>>();

        // This line triggers a compilation error
        test_send_trait(&fut);
    })
}
