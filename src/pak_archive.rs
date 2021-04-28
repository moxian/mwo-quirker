use std::io::Read;

struct IndexEntry {
    filename: String,
    data_offset: u64,
    data_len: u64,
}
pub struct PakArchive<T> {
    source: T,
    index: Vec<IndexEntry>,
}
impl<T> PakArchive<T>
where
    T: std::io::Read + std::io::Seek,
{
    pub fn new(mut source: T) -> std::io::Result<Self> {
        Ok(Self {
            index: read_index(&mut source)?,
            source,
        })
    }
    pub fn file_list(&self) -> Vec<String> {
        self.index.iter().map(|i| i.filename.to_string()).collect()
    }
    pub fn unpack(&mut self, filename: &str) -> Vec<u8> {
        let index_entry = self
            .index
            .iter()
            .find(|i| i.filename == filename)
            .ok_or_else(|| format!("File {:?} not present in archive", filename))
            .unwrap();
        self.source
            .seek(std::io::SeekFrom::Start(index_entry.data_offset))
            .unwrap();
        let mut compressed_data = vec![0; index_entry.data_len as usize];
        self.source.read_exact(&mut compressed_data).unwrap();
        let mut decoder = libflate::deflate::Decoder::new(&compressed_data[..]);
        let mut decoded_data = vec![];
        decoder.read_to_end(&mut decoded_data).unwrap();
        decoded_data
    }
}

fn read_index<T>(mut source: T) -> std::io::Result<Vec<IndexEntry>>
where
    T: std::io::Read + std::io::Seek,
{
    // let mut rdr = Reader::new(source);

    let mut index = Vec::new();

    loop {
        let magic = Reader2::read_bytes(&mut source, 4)?;
        match magic.as_slice() {
            b"PK\x03\x04" => { /*is a file*/ }
            b"PK\x01\x02" => {
                // is a central directory
                // we could parse that but nah, let's just break out
                break;
            }
            _ => {
                panic!("idk magic {:?}", magic)
            }
        }
        assert_eq!(magic.as_slice(), b"PK\x03\x04");
        let _ver = Reader2::read_bytes(&mut source, 2)?;
        let _opts = Reader2::read_bytes(&mut source, 2)?;
        let compression_method = Reader2::read_u16(&mut source)?;
        assert_eq!(compression_method, 8); // deflate
        let _last_modified_time = Reader2::read_u16(&mut source)?;
        let _last_modified_date = Reader2::read_u16(&mut source)?;

        let _crc = Reader2::read_bytes(&mut source, 4)?;
        let compressed_size = Reader2::read_u32(&mut source)?;
        let _uncompressed_size = Reader2::read_u32(&mut source)?;
        // println!("sizes {} {}", compressed_size, uncompressed_size);
        let filename_len = Reader2::read_u16(&mut source)?;
        let extra_field_len = Reader2::read_u16(&mut source)?;
        // println!("filenames {} {}", filename_len, extra_field_len);
        let filename =
            String::from_utf8(Reader2::read_bytes(&mut source, filename_len as usize)?).unwrap();
        let _extra_field = Reader2::read_bytes(&mut source, extra_field_len as usize)?;

        index.push(IndexEntry {
            filename,
            data_offset: std::io::Seek::seek(&mut source, std::io::SeekFrom::Current(0))?,
            data_len: compressed_size as u64,
        });
        std::io::Seek::seek(
            &mut source,
            std::io::SeekFrom::Current(compressed_size as i64),
        )?;
    }

    Ok(index)
}

// struct Reader<R> {
//     rdr: R,
// }
// impl<R: std::io::Read> Reader<R> {
//     fn new(rdr: R) -> Self {
//         Reader { rdr }
//     }
//     fn read_bytes(&mut self, count: usize) -> std::io::Result<Vec<u8>> {
//         let mut buf = vec![0; count];
//         self.rdr.read_exact(&mut buf)?;
//         Ok(buf)
//     }
//     fn read_u16(&mut self) -> std::io::Result<u16> {
//         let mut buf = [0u8; 2];
//         self.rdr.read_exact(&mut buf)?;
//         Ok(u16::from_le_bytes(buf))
//     }
//     fn read_u32(&mut self) -> std::io::Result<u32> {
//         let mut buf = [0u8; 4];
//         self.rdr.read_exact(&mut buf)?;
//         Ok(u32::from_le_bytes(buf))
//     }
// }

struct Reader2;
impl Reader2 {
    fn read_bytes<R: std::io::Read>(mut rdr: R, count: usize) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0; count];
        rdr.read_exact(&mut buf)?;
        Ok(buf)
    }
    fn read_u16<R: std::io::Read>(mut rdr: R) -> std::io::Result<u16> {
        let mut buf = [0u8; 2];
        rdr.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }
    fn read_u32<R: std::io::Read>(mut rdr: R) -> std::io::Result<u32> {
        let mut buf = [0u8; 4];
        rdr.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }
}
