impl WebStore {
    pub(crate) async fn insert_string(
        &mut self, 
        data: String
    ) -> Result<(), ()> {
        let result = JsFuture::from(insert_greeting(data)).await;
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}