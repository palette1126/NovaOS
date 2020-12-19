extern crate easy_fs;
extern crate alloc;

use easy_fs::{
    BlockDevice,
    EasyFileSystem,
};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::sync::Mutex;
use alloc::sync::Arc;
use rand;

const BLOCK_SZ: usize = 512;

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        println!("reading block {}", block_id);
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        println!("writing block {}", block_id);
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }
}

fn main() {
    easy_fs_pack().expect("Error when packing easy-fs!");
}

fn easy_fs_pack() -> std::io::Result<()> {
    Ok(())
}

#[test]
fn efs_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile(Mutex::new(
        OpenOptions::new()
            .read(true)
            .write(true)
            .open("target/fs.img")?
    )));
    EasyFileSystem::create(
        block_file.clone(),
        4096,
        1,
    );
    let efs = EasyFileSystem::open(block_file.clone());
    let mut root_inode = EasyFileSystem::root_inode(&efs);
    root_inode.create("filea");
    root_inode.create("fileb");
    for name in root_inode.ls() {
        println!("{}", name);
    }
    let filea = root_inode.find("filea").unwrap();
    let greet_str = "Hello, world!";
    filea.write_at(0, greet_str.as_bytes());
    let mut buffer = [0u8; 512];
    let len = filea.read_at(0, &mut buffer);
    assert_eq!(
        greet_str,
        core::str::from_utf8(&buffer[..len]).unwrap(),
    );

    let mut random_str_test = |len: usize| {
        filea.clear();
        assert_eq!(
            filea.read_at(0, &mut buffer),
            0,
        );
        let mut str = String::new();
        // random digit
        for _ in 0..len {
            str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
        }
        filea.write_at(0, str.as_bytes());
        let mut read_buffer = [0u8; 127];
        let mut offset = 0usize;
        let mut read_str = String::new();
        loop {
            let len = filea.read_at(offset, &mut read_buffer);
            if len == 0 {
                break;
            }
            offset += len;
            read_str.push_str(
                core::str::from_utf8(&read_buffer[..len]).unwrap()
            );
        }
        assert_eq!(str, read_str);
    };

    random_str_test(4 * BLOCK_SZ);
    random_str_test(8 * BLOCK_SZ + BLOCK_SZ / 2);
    random_str_test(100 * BLOCK_SZ);
    random_str_test(70 * BLOCK_SZ + BLOCK_SZ / 7);
    random_str_test((12 + 128) * BLOCK_SZ);

    Ok(())
}