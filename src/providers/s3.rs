extern crate quick_xml;
extern crate serde;

use std::error::Error;

use chrono::{DateTime, Utc};
use rusoto_core::{credential::ProfileProvider, ByteStream, HttpClient, Region, RusotoError};
use rusoto_s3::{
    DeleteObjectRequest, GetObjectOutput, GetObjectRequest, ListObjectsV2Request, PutObjectRequest,
    S3Client, S3,
};
use serde::Deserialize;

use super::Kind;

#[derive(Debug, Deserialize)]
pub struct S3Error {
    #[serde(rename = "Code", default)]
    code: String,
    #[serde(rename = "Message", default)]
    message: String,
}

impl S3Error {
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone)]
pub struct S3Object {
    pub name: String,
    pub prefix: String,
    pub kind: Kind,
    pub size: Option<i64>,
    pub last_mod: DateTime<Utc>,
    pub storage_class: Option<String>,
    pub owner: Option<String>,
}

pub struct S3Provider {
    pub bucket_name: String,
    s3_client: S3Client,
}

impl S3Provider {
    fn handle_error(err: RusotoError<impl Error>) -> S3Error {
        match err {
            RusotoError::Unknown(buf) => {
                let text = buf.body_as_str();
                let s3err: S3Error = quick_xml::de::from_str(text).unwrap();
                s3err
            }
            RusotoError::HttpDispatch(err) => S3Error {
                code: "Request Error".to_string(),
                message: err.to_string(),
            },
            RusotoError::Credentials(err) => S3Error {
                code: "Credentials Error".to_string(),
                message: err.to_string(),
            },
            RusotoError::Validation(msg) => S3Error {
                code: "Validation Error".to_string(),
                message: msg,
            },
            RusotoError::ParseError(msg) => S3Error {
                code: "ParsingError".to_string(),
                message: msg,
            },
            _ => S3Error {
                code: "Unknown Error".to_string(),
                message: "Unknown error occured".to_string(),
            },
        }
    }

    pub async fn new(bucket_name: &str) -> S3Provider {
        S3Provider {
            bucket_name: String::from(bucket_name),
            s3_client: S3Client::new_with(
                HttpClient::new().unwrap(),
                ProfileProvider::new()
                    .expect("Please provide your aws credentials in the .aws file"),
                Region::EuCentral1,
            ),
        }
    }

    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<S3Object>, S3Error> {
        let mut request = ListObjectsV2Request::default();
        request.bucket = self.bucket_name.clone();
        request.prefix = if prefix.is_empty() {
            None
        } else {
            Some(String::from(prefix))
        };
        let objects = self.s3_client.list_objects_v2(request);
        let response = match objects.await.map_err(Self::handle_error)?.contents {
            None => return Ok(Vec::new()),
            Some(contents) => contents,
        };
        let result = response
            .into_iter()
            .filter(|i| {
                let key = i.key.clone().unwrap();
                let (prefix, file_name) = key.split_at(prefix.len());
                // Ensures function returns only top-level files and directories
                // for given prefix. (entries like foo/bar.txt are ommited)
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
                    size: i.size,
                    last_mod: DateTime::parse_from_rfc3339(i.last_modified.unwrap().as_str())
                        .expect("Couldn't parse object's last modification date from string")
                        .with_timezone(&Utc),
                    storage_class: i.storage_class,
                    owner: match i.owner {
                        Some(own) => own.display_name,
                        None => None,
                    },
                }
            })
            .collect();
        Ok(result)
    }

    pub async fn download_object(&self, object_name: &str) -> Result<ByteStream, S3Error> {
        let object: GetObjectOutput = self.get_object(object_name).await?;
        Ok(object.body.unwrap())
    }

    async fn get_object(&self, object_name: &str) -> Result<GetObjectOutput, S3Error> {
        let mut request = GetObjectRequest::default();
        request.bucket = self.bucket_name.clone();
        request.key = String::from(object_name);

        Ok(self
            .s3_client
            .get_object(request)
            .await
            .map_err(Self::handle_error)?)
    }

    pub async fn delete_object(&self, object_name: &str) -> Result<(), S3Error> {
        let mut request = DeleteObjectRequest::default();
        request.bucket = self.bucket_name.clone();
        request.key = String::from(object_name);
        self.s3_client
            .delete_object(request)
            .await
            .map_err(Self::handle_error)?;
        Ok(())
    }

    pub async fn put_object(&self, object_name: &str, content: ByteStream) -> Result<(), S3Error> {
        let mut request = PutObjectRequest::default();
        request.bucket = self.bucket_name.clone();
        request.key = String::from(object_name);
        request.body = Some(content);

        self.s3_client
            .put_object(request)
            .await
            .map_err(Self::handle_error)?;
        Ok(())
    }
}
