use std::{path::Path, fs::File, io::{Write}};

use chrono::{DateTime, Utc};
use rusoto_core::{credential::{ProfileProvider}, Region, HttpClient};
use rusoto_s3::{S3Client, S3, ListObjectsV2Request, GetObjectRequest, GetObjectOutput};
use tokio::io::AsyncReadExt;

#[derive(Debug)]
pub struct S3Object {
    pub name: String,
    pub size: i64,
    pub last_mod: DateTime<Utc>,
    pub storage_class: String,
    pub owner: Option<String>,
}
pub struct Cli {
    s3_client: S3Client,
}

impl Cli {

    pub async fn new() -> Cli {
        Cli {
            s3_client: S3Client::new_with(
                    HttpClient::new().unwrap(),
                    ProfileProvider::new().unwrap(),
                    Region::UsEast1
                )
        }
    }

    pub async fn list_objects(&self, bucket_name: &str) -> Vec<S3Object> {
        let mut request = ListObjectsV2Request::default();
        request.bucket = String::from(bucket_name);
        let objects = self.s3_client.list_objects_v2(
            request
        );
        let response = objects.await.unwrap().contents.unwrap();
        response.into_iter().map(|i| {
                S3Object {
                    name: i.key.unwrap(),
                    size: i.size.unwrap(),
                    last_mod: DateTime::parse_from_rfc3339(i.last_modified.unwrap().as_str()).unwrap().with_timezone(&Utc),
                    storage_class: i.storage_class.unwrap(),
                    owner: match i.owner {
                        Some(own) => own.display_name,
                        None => None
                    }
                }
            }).collect()
    }

    pub async fn download_object(&self, bucket_name: &str, object_name: &str, path: &Path) {
        let object: GetObjectOutput = self.get_object(bucket_name, object_name).await;

        let mut file = File::create(&path).unwrap();

        let content_length: usize =  object.content_length.unwrap() as usize;
        let mut content = object.body.unwrap().into_async_read();

        let mut output: Vec<u8> = Vec::with_capacity(content_length);
        content.read_to_end(&mut output).await.unwrap();

        file.write_all(&output).unwrap()
    }

    async fn get_object(&self, bucket_name: &str, object_name: &str) -> GetObjectOutput {
        let mut request = GetObjectRequest::default();
        request.bucket = String::from(bucket_name);
        request.key = String::from(object_name);
        let response = self.s3_client.get_object(request).await.unwrap();
        response
    }
}


#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Cli;
    const BUCKET_NAME: &str = "test-bucket";

    #[tokio::test]
    async fn list_objects_from_bucket() {
        let cli = Cli::new().await;
        let _objects = cli.list_objects(BUCKET_NAME).await;
    }

    #[tokio::test]
    async fn get_object_from_bucket() {
        let cli = Cli::new().await;
        cli.download_object(BUCKET_NAME, "photo_2021-04-25_16-12-37.jpg", Path::new("photo_2021-04-25_16-12-37.jpg")).await;
    }
}