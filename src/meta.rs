// This file is part of MinSQL
// Copyright (c) 2019 MinIO, Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::process;
use std::sync::{Arc, RwLock};

use futures::future::Future;
use futures::stream;
use futures::Stream;
use log::error;
use rusoto_s3::{GetObjectRequest, ListObjectsRequest, S3};

use crate::config::{Config, DataStore, Log, LogAuth};
use crate::storage;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub struct Meta {
    config: Arc<RwLock<Config>>,
}

impl Meta {
    pub fn new(cfg: Arc<RwLock<Config>>) -> Meta {
        Meta { config: cfg }
    }

    /// Scans the metabucket for configuration files and loads them into the shared state `Config`
    pub fn load_config_from_metabucket(&self) -> impl Future<Item = (), Error = ()> {
        // validate access to the metadata store
        let ds = ds_for_metabucket(Arc::clone(&self.config));
        match storage::can_reach_datastore(&ds) {
            Ok(true) => (),
            Ok(false) => {
                println!("Metabucket is not reachable");
                process::exit(0x0100);
            }
            Err(e) => match e {
                storage::StorageError::Operation(
                    storage::ReachableDatastoreError::NoSuchBucket(s),
                ) => {
                    println!("Metabucket doesn't exists: {:?}", s);
                    process::exit(0x0100);
                }
                _ => {
                    println!("Metabucket is not reachable");
                    process::exit(0x0100);
                }
            },
        }

        // Create s3 client
        let s3_client = storage::client_for_datastore(&ds);
        let s3_client = Arc::new(s3_client);

        let s3_client1 = Arc::clone(&s3_client);
        let s3_client2 = Arc::clone(&s3_client);

        let main_cfg = Arc::clone(&self.config);
        // get all the objects inside the meta folder
        let task = s3_client1
            .list_objects(ListObjectsRequest {
                bucket: ds.bucket.clone(),
                prefix: Some("minsql/meta/".to_owned()),
                ..Default::default()
            })
            .map(|list_objects| match list_objects.contents {
                Some(v) => v,
                None => Vec::new(),
            })
            .map_err(|_| ())
            .and_then(|objects| {
                // For each objects, get_object, filter out system files
                stream::iter_ok(objects)
                    .map(|file_object| file_object.clone().key.unwrap())
                    .map(move |file_key| {
                        let file_key_clone = file_key.clone();
                        s3_client2
                            .get_object(GetObjectRequest {
                                bucket: ds.bucket.clone(),
                                key: file_key,
                                ..Default::default()
                            })
                            .map_err(|e| {
                                error!("getting object: {:?}", e);
                                ()
                            })
                            .and_then(|object_output| {
                                // Deserialize the object output and wrap in an `MetaConfigObject`
                                object_output
                                    .body
                                    .unwrap()
                                    .concat2()
                                    .map_err(|e| {
                                        error!("concatenating body: {:?}", e);
                                        ()
                                    })
                                    .map(move |bytes| {
                                        let result = String::from_utf8(bytes.to_vec()).unwrap();
                                        let parts: Vec<&str> = file_key_clone
                                            .trim_start_matches("minsql/meta/")
                                            .split("/")
                                            .collect();
                                        let meta_obj = match (parts.len(), parts[0]) {
                                            (2, "logs") => match serde_json::from_str(&result) {
                                                Ok(t) => MetaConfigObject::Log(t),
                                                Err(_) => MetaConfigObject::Unknown,
                                            },
                                            (2, "datastores") => {
                                                match serde_json::from_str(&result) {
                                                    Ok(t) => MetaConfigObject::DataStore(t),
                                                    Err(_) => MetaConfigObject::Unknown,
                                                }
                                            }
                                            (3, "auth") => match serde_json::from_str(&result) {
                                                Ok(t) => MetaConfigObject::LogAuth((
                                                    parts[1].to_string(),
                                                    parts[2].to_string(),
                                                    t,
                                                )),
                                                Err(_) => MetaConfigObject::Unknown,
                                            },
                                            _ => MetaConfigObject::Unknown,
                                        };
                                        meta_obj
                                    })
                            })
                    })
                    .buffer_unordered(5)
                    .collect()
            })
            .map_err(|e| {
                println!("mapping contents to structs:  {:?}", e);
                ()
            })
            .map(move |result_meta_objects: Vec<MetaConfigObject>| {
                //get a write lock on config
                let mut cfg = main_cfg.write().unwrap();
                //time to update the configuration!
                for mco in result_meta_objects {
                    match mco {
                        MetaConfigObject::Log(l) => {
                            cfg.log.insert(l.clone().name.unwrap(), l);
                        }
                        MetaConfigObject::DataStore(ds) => {
                            cfg.datastore.insert(ds.clone().name.unwrap(), ds);
                        }
                        MetaConfigObject::LogAuth((token, log_name, log_auth)) => {
                            // Get the map for the token, if it's not set yet, initialize it.
                            let auth_logs = match cfg.auth.entry(token) {
                                Entry::Occupied(o) => o.into_mut(),
                                Entry::Vacant(v) => v.insert(HashMap::new()),
                            };
                            auth_logs.insert(log_name, log_auth);
                        }
                        _ => (),
                    }
                }
            })
            .map(|_| ());
        task
    }
}

pub fn ds_for_metabucket(cfg: Arc<RwLock<Config>>) -> DataStore {
    // TODO: Maybe cache this on cfg.server
    let read_cfg = cfg.read().unwrap();
    // Represent the metabucket as a datastore to re-use other functions we have in `storage.rs`
    DataStore {
        endpoint: read_cfg.server.metadata_endpoint.clone(),
        access_key: read_cfg.server.access_key.clone(),
        secret_key: read_cfg.server.secret_key.clone(),
        bucket: read_cfg.server.metadata_bucket.clone(),
        prefix: "".to_owned(),
        name: Some("metabucket".to_owned()),
    }
}

#[derive(Debug)]
enum MetaConfigObject {
    Log(Log),
    DataStore(DataStore),
    LogAuth((String, String, LogAuth)),
    Unknown,
}
