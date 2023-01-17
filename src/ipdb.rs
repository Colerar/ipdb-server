use std::{
  collections::{BTreeMap, HashMap},
  net::IpAddr,
};

use anyhow::{bail, ensure, Context, Result};
use memmap2::Mmap;
use serde::Deserialize;

pub struct Reader<'a> {
  data: &'a [u8],
  pub metadata: Metadata,
  v4offset: usize,
}

#[derive(Deserialize)]
pub struct Metadata {
  pub build: i32,
  pub ip_version: i32,
  pub node_count: usize,
  pub languages: HashMap<String, usize>,
  pub fields: Vec<String>,
  pub total_size: usize,
}

impl<'a> Reader<'a> {
  pub fn new(inner: &'a Mmap) -> Result<Self> {
    let meta_len = u32::from_be_bytes((inner[..4]).try_into()?) as usize + 4;
    let metadata: Metadata =
      serde_json::from_slice(&inner[4..meta_len]).context("Failed to deserialize ipdb metadata")?;
    ensure!(
      metadata.total_size + meta_len == inner.len(),
      "database file size error"
    );
    let data = &inner[meta_len..];
    let mut node = 0usize;
    for i in 0..96 {
      if node >= metadata.node_count {
        break;
      }
      if i >= 80 {
        let off = node * 8 + 1 * 4;
        node = u32::from_be_bytes((&data[off..off + 4]).try_into()?) as usize;
      } else {
        let off = node * 8;
        node = u32::from_be_bytes((&data[off..off + 4]).try_into()?) as usize;
      }
    }

    Ok(Reader {
      data: &inner[meta_len..],
      metadata,
      v4offset: node,
    })
  }

  fn resolve(&self, node: usize) -> Result<&str> {
    let resolved = node - self.metadata.node_count + self.metadata.node_count * 8;
    ensure!(
      resolved < self.data.len(),
      "database resolve error, resolved:{} > file length:{}",
      resolved,
      self.data.len()
    );
    let off = u32::from_be_bytes([0u8, 0u8, self.data[resolved], self.data[resolved + 1]]) as usize;
    let size = off + resolved + 2;
    ensure!(
      self.data.len() > size,
      "database resolve error,size:{}>file length:{}",
      size,
      self.data.len()
    );
    unsafe {
      Ok(std::str::from_utf8_unchecked(
        &self.data[resolved + 2..size],
      ))
    }
  }

  fn read_node(&self, node: usize, index: usize) -> Result<usize> {
    let off = node * 8 + index * 4;
    Ok(u32::from_be_bytes((&self.data[off..off + 4]).try_into()?) as usize)
  }

  fn find_node(&self, binary: &[u8]) -> Result<usize> {
    let mut node = 0;
    let bit = binary.len() * 8;
    if bit == 32 {
      node = self.v4offset;
    }
    for i in 0..bit {
      if node > self.metadata.node_count {
        return Ok(node);
      }
      node = self.read_node(node, (1 & ((0xFF & binary[i / 8]) >> 7 - (i % 8))) as usize)?;
    }

    if node > self.metadata.node_count {
      return Ok(node);
    } else {
      bail!("not found ip")
    }
  }

  pub fn is_ipv4(&self) -> bool {
    self.metadata.ip_version & 0x01 == 0x01
  }

  pub fn is_ipv6(&self) -> bool {
    self.metadata.ip_version & 0x02 == 0x02
  }

  pub fn find(&self, addr: IpAddr, language: &str) -> Result<Vec<&str>> {
    ensure!(!self.metadata.fields.is_empty(), "fields is empty");
    let off = *self
      .metadata
      .languages
      .get(language)
      .with_context(|| format!("Language does not exist: {}", language))?;
    let node = match &addr {
      IpAddr::V4(v) => {
        ensure!(self.is_ipv4(), "ipdb is ipv6");
        self.find_node(&v.octets())
      }
      IpAddr::V6(v) => {
        ensure!(self.is_ipv6(), "ipdb is ipv4");
        self.find_node(&v.octets())
      }
    }?;
    let context = self.resolve(node)?;
    let sp: Vec<&str> = context.split('\t').skip(off).collect();
    Ok(sp)
  }

  pub fn find_to_map(&self, addr: IpAddr, language: &str) -> Result<BTreeMap<String, String>> {
    let values = self.find(addr, language)?;
    let vec: Vec<(String, String)> = self
      .metadata
      .fields
      .clone()
      .into_iter()
      .zip(values.into_iter().map(|i| i.to_string()))
      .collect();
    let mut map: BTreeMap<String, String> = BTreeMap::from_iter(vec.into_iter());
    map.retain(|_, v| !v.is_empty());
    Ok(map)
  }
}
