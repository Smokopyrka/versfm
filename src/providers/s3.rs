use chrono::{DateTime, Utc};
use rusoto_core::{credential::ProfileProvider, ByteStream, HttpClient, Region};
use rusoto_s3::{
    DeleteObjectRequest, GetObjectOutput, GetObjectRequest, ListObjectsV2Request, PutObjectRequest,
    S3Client, S3,
};

use crate::view::components::FileEntry;

use super::Kind;

#[derive(Clone)]
pub struct S3Object {
    pub name: String,
    pub prefix: String,
    pub kind: Kind,
    pub size: i64,
    pub last_mod: DateTime<Utc>,
    pub storage_class: String,
    pub owner: Option<String>,
}

impl FileEntry for S3Object {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_kind(&self) -> &Kind {
        &self.kind
    }
}

pub struct S3Provider {
    pub bucket_name: String,
    s3_client: S3Client,
}

impl S3Provider {
    pub async fn new(bucket_name: &str) -> S3Provider {
        S3Provider {
            bucket_name: String::from(bucket_name),
            s3_client: S3Client::new_with(
                HttpClient::new().unwrap(),
                ProfileProvider::new().unwrap(),
                Region::EuCentral1,
            ),
        }
    }

    pub async fn list_objects(&self, prefix: Option<String>) -> Vec<S3Object> {
        let mut request = ListObjectsV2Request::default();
        request.bucket = self.bucket_name.clone();
        request.prefix = prefix.clone();
        let objects = self.s3_client.list_objects_v2(request);
        let response = match objects.await.unwrap().contents {
            None => return Vec::new(),
            Some(contents) => contents,
        };
        let prefix = prefix.unwrap_or(String::new());
        response
            .into_iter()
            .filter(|i| {
                let key = i.key.clone().unwrap();
                let (prefix, file_name) = key.split_at(prefix.len());
                match (prefix, file_name) {
                    ("", name) => match name.find("/") {
                        None => true,
                        Some(i) => i == name.len() - 1,
                    },
                    (_, "") => false,
                    (_, name) => {
                        let last_char = name.chars().last().unwrap();
                        let seperator_count = name.matches('/').count();
                        seperator_count == 0 || (seperator_count == 1 && last_char == '/')
                    }
                }
            })
            .map(|i| {
                let key = i.key.clone().unwrap();
                let (prefix, file_name) = key.split_at(prefix.len());
                let kind: Kind;
                if file_name.chars().last().unwrap() == '/' {
                    kind = Kind::Directory;
                } else {
                    kind = Kind::File;
                }
                S3Object {
                    name: String::from(file_name),
                    prefix: String::from(prefix),
                    kind,
                    size: i.size.unwrap(),
                    last_mod: DateTime::parse_from_rfc3339(i.last_modified.unwrap().as_str())
                        .unwrap()
                        .with_timezone(&Utc),
                    storage_class: i.storage_class.unwrap(),
                    owner: match i.owner {
                        Some(own) => own.display_name,
                        None => None,
                    },
                }
            })
            .collect()
    }

    pub async fn download_object(&self, object_name: &str) -> ByteStream {
        let object: GetObjectOutput = self.get_object(object_name).await;
        object.body.unwrap()
    }

    async fn get_object(&self, object_name: &str) -> GetObjectOutput {
        let mut request = GetObjectRequest::default();
        request.bucket = self.bucket_name.clone();
        request.key = String::from(object_name);
        let response = self.s3_client.get_object(request).await.unwrap();
        response
    }

    pub async fn delete_object(&self, object_name: &str) {
        let mut request = DeleteObjectRequest::default();
        request.bucket = self.bucket_name.clone();
        request.key = String::from(object_name);
        self.s3_client.delete_object(request).await.unwrap();
    }

    pub async fn put_object(&self, object_name: &str, content: ByteStream) {
        // let mut file = File::open(file_path).unwrap();
        // let mut contents: Vec<u8> = Vec::new();
        // file.read_to_end(&mut contents).unwrap();

        let mut request = PutObjectRequest::default();
        request.bucket = self.bucket_name.clone();
        request.key = String::from(object_name);
        // request.body = Some(ByteStream::from(contents));
        request.body = Some(content);

        self.s3_client.put_object(request).await.unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::S3Provider;
    const BUCKET_NAME: &str = "s3tui-test-bucket";

    #[tokio::test]
    async fn list_objects_from_bucket() {
        let cli = S3Provider::new(BUCKET_NAME).await;
        let _objects = cli.list_objects(None).await;
    }

    // #[tokio::test]
    // async fn get_object_from_bucket() {
    //     let cli = Cli::new(BUCKET_NAME).await;
    //     cli.download_object("get-object-test.txt", Path::new("get-object-test.txt"))
    //         .await;
    // }

    #[tokio::test]
    async fn remove_item_from_bucket() {
        let cli = S3Provider::new(BUCKET_NAME).await;
        cli.delete_object("delete-object-test.txt").await;
    }

    // #[tokio::test]
    // async fn put_item_into_bucket() {
    //     let cli = Cli::new(BUCKET_NAME).await;
    //     cli.put_object("put-object-test.txt", Path::new("put-object-test.txt"))
    //         .await;
    // }
}
