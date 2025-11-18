use uuid::Uuid;

pub struct Document {
    pub uuid: Uuid,
    pub content: String,
    pub version: u32,
}

impl Document {
    pub fn apply_operation(
        &mut self,
        new_content: String,
        client_version: u32,
    ) -> Result<(String, u32), String> {
        if self.version != client_version {
            return Err(format!(
                "Version mismatch: document={}, client={}",
                self.version, client_version
            ));
        }
        if new_content.is_empty() {
            return Err("Operation content cannot be empty".to_string());
        }

        // Apply the operation
        self.content = new_content;
        self.version += 1;

        Ok((self.content.clone(), self.version))
    }
}
