use async_std::{fs, task};

#[test]
fn open_file() {
    task::block_on(async {
        let non_existing_file = "/ashjudlkahasdasdsikdhajik/asdasdasdasdasdasd/fjuiklashdbflasas";
        let res = fs::File::open(non_existing_file).await;
        match res {
            Ok(_) => panic!("Found file with random name: We live in a simulation"),
            Err(e) => assert_eq!(
                "Could not open `/ashjudlkahasdasdsikdhajik/asdasdasdasdasdasd/fjuiklashdbflasas`",
                &format!("{}", e)
            ),
        }
    })
}
