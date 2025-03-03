#![warn(missing_docs)]
#![allow(clippy::ptr_offset_with_cast)]
//! Persistence Storage for default, use `sled` as backend db.
use async_trait::async_trait;
use itertools::Itertools;
use serde::de::DeserializeOwned;
use sled;

use super::PersistenceStorageOperation;
use super::PersistenceStorageReadAndWrite;
use super::PersistenceStorageRemove;
use crate::err::Error;
use crate::err::Result;

trait KvStorageBasic {
    fn get_db(&self) -> &sled::Db;
}

/// StorageInstance struct
pub struct KvStorage {
    db: sled::Db,
    cap: usize,
}

impl KvStorage {
    /// New KvStorage
    /// * cap: max_size in bytes
    /// * path: db file location
    pub async fn new_with_cap_and_path<P>(cap: usize, path: P) -> Result<Self>
    where P: AsRef<std::path::Path> {
        let db = sled::Config::new()
            .path(path)
            .mode(sled::Mode::HighThroughput)
            .cache_capacity(cap as u64)
            .open()
            .map_err(Error::SledError)?;
        Ok(Self { db, cap })
    }

    /// New KvStorage with default path
    /// default_path is `./`
    pub async fn new_with_cap(cap: usize) -> Result<Self> {
        Self::new_with_cap_and_path(cap, "./data").await
    }

    /// New KvStorage
    /// * default capacity 200 megabytes
    /// * default path `./data`
    pub async fn new() -> Result<Self> {
        Self::new_with_cap(200000000).await
    }
}

impl KvStorageBasic for KvStorage {
    fn get_db(&self) -> &sled::Db {
        &self.db
    }
}

#[async_trait]
impl PersistenceStorageOperation for KvStorage {
    async fn clear(&self) -> Result<()> {
        self.db.clear().map_err(Error::SledError)?;
        // self.db.flush_async().await.map_err(Error::SledError)?;
        Ok(())
    }

    async fn count(&self) -> Result<u64> {
        Ok(self.db.len() as u64)
    }

    async fn max_size(&self) -> Result<usize> {
        Ok(self.cap)
    }

    /// Get the storage size, if applicable.
    async fn total_size(&self) -> Result<usize> {
        Ok(self.db.len())
    }

    /// Prune database storage
    async fn prune(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl<K, V, I> PersistenceStorageReadAndWrite<K, V> for I
where
    K: ToString + From<String> + std::marker::Sync + Send,
    V: DeserializeOwned + serde::Serialize + std::marker::Sync + Send,
    I: PersistenceStorageOperation + std::marker::Sync + KvStorageBasic,
{
    /// Get a cache entry by `key`.
    async fn get(&self, key: &K) -> Result<V> {
        let k = key.to_string();
        let k = k.as_bytes();
        let v = self
            .get_db()
            .get(k)
            .map_err(Error::SledError)?
            .ok_or(Error::EntryNotFound)?;
        bincode::deserialize(v.as_ref()).map_err(Error::BincodeDeserialize)
    }

    /// Put `entry` in the cache under `key`.
    async fn put(&self, key: &K, value: &V) -> Result<()> {
        self.prune().await?;
        let data = bincode::serialize(value).map_err(Error::BincodeSerialize)?;
        self.get_db()
            .insert(key.to_string().as_bytes(), data)
            .map_err(Error::SledError)?;
        Ok(())
    }

    async fn get_all(&self) -> Result<Vec<(K, V)>> {
        let iter = self.get_db().iter();
        Ok(iter
            .flatten()
            .flat_map(|(k, v)| {
                Some((
                    K::from(std::str::from_utf8(k.as_ref()).ok()?.to_string()),
                    bincode::deserialize(v.as_ref()).ok()?,
                ))
            })
            .collect_vec())
    }
}

#[async_trait]
impl<K, I> PersistenceStorageRemove<K> for I
where
    K: ToString + std::marker::Sync,
    I: PersistenceStorageOperation + std::marker::Sync + KvStorageBasic,
{
    async fn remove(&self, key: &K) -> Result<()> {
        self.get_db()
            .remove(key.to_string().as_bytes())
            .map_err(Error::SledError)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use serde::Deserialize;
    use serde::Serialize;

    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    struct TestStorageStruct {
        content: String,
    }

    #[tokio::test]
    async fn test_kv_storage_put_delete() {
        let storage = KvStorage::new_with_cap_and_path(4096, "temp/db")
            .await
            .unwrap();
        let key1 = "test1".to_owned();
        let data1 = TestStorageStruct {
            content: "test1".to_string(),
        };
        storage.put(&key1, &data1).await.unwrap();
        let count1 = storage.count().await.unwrap();
        assert!(count1 == 1, "expect count1.1 is {}, got {}", 1, count1);
        let got_v1: TestStorageStruct = storage.get(&key1).await.unwrap();
        assert!(
            got_v1.content.eq(data1.content.as_str()),
            "expect value1 is {}, got {}",
            data1.content,
            got_v1.content
        );

        let key2 = "test2".to_owned();
        let data2 = TestStorageStruct {
            content: "test2".to_string(),
        };

        storage.put(&key2, &data2).await.unwrap();

        let count_got_2 = storage.count().await.unwrap();
        assert!(count_got_2 == 2, "expect count 2, got {}", count_got_2);

        let all_entries: Vec<(String, TestStorageStruct)> = storage.get_all().await.unwrap();
        assert!(
            all_entries.len() == 2,
            "all_entries len expect 2, got {}",
            all_entries.len()
        );

        let keys = vec![key1, key2];
        let values = vec![data1.content, data2.content];

        assert!(
            all_entries
                .iter()
                .any(|(k, v)| { keys.contains(k) && values.contains(&v.content) }),
            "not found items"
        );

        storage.clear().await.unwrap();
        let count1 = storage.count().await.unwrap();
        assert!(count1 == 0, "expect count1.2 is {}, got {}", 0, count1);
        storage.get_db().flush_async().await.unwrap();
        drop(storage)
    }
}
