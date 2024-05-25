// TODO: remove
#![allow(dead_code)]
use std::{fmt::write, io::Read, path::Path};

use libflate::zlib::Decoder;

#[derive(Debug, PartialEq, Eq)]
enum Error {
    Reader,
    InvalidFile,
    InvalidPath,
    UnknownChunkType,
    EndOfFile,
    ReservedBlockEncoding,
    Decoding,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::Reader => "Reader could not read exact bytes from buffer",
            Self::InvalidFile => "The given file is not a PNG",
            Self::InvalidPath => "There is no such file at the given path",
            Self::UnknownChunkType => "This PNG format is unimplemented",
            Self::EndOfFile => "na",
            Self::ReservedBlockEncoding => "Decoding error, this is a bug",
            Self::Decoding => "Failed to decode data, this is a bug",
        };
        f.write_str(msg)
    }
}

#[derive(Debug)]
enum PixelType {
    IndexedColor,
    GrayScale,
    TrueColor,
}

#[derive(Debug)]
struct PNGParser {
    format: PNGFormat,
    pixel_type: PixelType,
    chunks: Vec<Chunk>,
}

impl PNGParser {
    pub fn new(mut reader: Reader<std::fs::File>) -> Result<Self, Error> {
        // TODO: support different formats

        let ihdr_chunk = Chunk::new(&mut reader)?;
        let format = PNGFormat::from_ihdr_chunk(ihdr_chunk)?;

        let mut chunks = Vec::new();
        loop {
            let next_chunk = Chunk::new(&mut reader);
            match next_chunk {
                Ok(next_chunk) => chunks.push(next_chunk),
                Err(err) => {
                    if err == Error::EndOfFile {
                        break;
                    } else {
                        return Err(err);
                    }
                }
            }
        }

        let indexed_color = chunks
            .iter()
            .find(|c| c.chunk_type == ChunkType::PLTE)
            .is_some();
        let grayscale = indexed_color && format.color_type == 0;

        let pixel_type = if indexed_color {
            PixelType::IndexedColor
        } else if grayscale {
            PixelType::GrayScale
        } else {
            PixelType::TrueColor
        };

        Ok(Self {
            format,
            chunks,
            pixel_type,
        })
    }

    pub fn parse_to_bytes(self) -> Result<Vec<u8>, Error> {
        // TODO: preallocate vector
        let mut bytes = Vec::new();

        println!("{:?}", self);

        for mut chunk in self.chunks.into_iter() {
            match chunk.chunk_type {
                ChunkType::PLTE => {
                    let palette = chunk.reader.read_bytes(chunk.length as usize * 3)?;
                    println!("palette: {palette:?}");
                }
                ChunkType::pHYs => {
                    let ppux = chunk.reader.read_u32()?;
                    let ppuy = chunk.reader.read_u32()?;
                    let unit = chunk.reader.read_u8()?;
                    println!("ppux: {ppux}, ppuy: {ppuy}, unit: {unit}");
                }
                ChunkType::iCCP => {
                    let profile_name = chunk.reader.read_null_terminated_string()?;
                    let compression_method = chunk.reader.read_u8()?;
                    let compressed_profile =
                        chunk.reader.read_remaining_bytes(chunk.length as usize)?;
                    println!("profile_name: {profile_name:?}, compression_method: {compression_method}, compressed_profile: {compressed_profile:?}");
                }
                ChunkType::tIME => {
                    let year = chunk.reader.read_u16()?;
                    let month = chunk.reader.read_u8()?;
                    let day = chunk.reader.read_u8()?;
                    let hour = chunk.reader.read_u8()?;
                    let minute = chunk.reader.read_u8()?;
                    let second = chunk.reader.read_u8()?;
                    println!("year: {year}, month: {month}, day: {day}, hour: {hour}, minute: {minute}, second: {second}");
                }
                ChunkType::tEXt => {
                    let keyword = chunk.reader.read_null_terminated_string()?;
                    let text = chunk.reader.read_remaining_bytes(chunk.length as usize)?;
                    let text = text.into_iter().map(|b| b as char).collect::<String>();
                    println!("keyword: {keyword}, text: {text}")
                }
                ChunkType::IDAT => {
                    // Scanlines always begin on byte boundaries. When pixels have fewer than 8 bits and the scanline width is not evenly divisible by the
                    // number of pixels per byte, the low-order bits in the last byte of each scanline are wasted. The contents of these wasted bits are unspecified.

                    // let compression_method = chunk.reader.read_u8()?;
                    // let additional_flags = chunk.reader.read_u8()?;
                    // let compressed_data = chunk.reader.read_bytes(chunk.length as usize - 6)?;
                    // let check_value = chunk.reader.read_u32()?;

                    // println!("{compressed_data:?}");
                    // println!("{}", compressed_data.len());

                    let compressed_data = chunk.reader.read_bytes(chunk.length as usize)?;
                    let mut decoder = Decoder::new(compressed_data.as_slice()).map_err(|err| {
                        println!("{err}");
                        Error::Decoding
                    })?;
                    let mut decoded_data = Vec::new();
                    decoder.read_to_end(&mut decoded_data).map_err(|err| {
                        println!("{err}");
                        Error::Decoding
                    })?;

                    // println!("{compressed_data:?}");
                    println!("{}", decoded_data.len());

                    // let mut b_final = false;
                    // loop {
                    //     let block_header = data_reader.read_u8()?;
                    //     if block_header & 0x01 == 0x01 {
                    //         b_final = true;
                    //     }

                    //     match (block_header >> 1) & 0b00000011 {
                    //         // No compression
                    //         0b00 => {
                    //             let len = data_reader.read_u16()?;
                    //             let _ = data_reader.read_u16()?;
                    //             let block_data = data_reader.read_bytes(len as usize)?;
                    //             println!("Block data: {block_data:?}");
                    //         }
                    //         // Compressed with fixed Huffman codes
                    //         0b01 => {}
                    //         // Compressed with dynamic Huffman codes
                    //         0b10 => {}
                    //         0b11 => return Err(Error::ReservedBlockEncoding),
                    //         _ => unreachable!(),
                    //     }

                    //     if b_final {
                    //         break;
                    //     }
                    // }

                    // println!("compression_method: {compression_method}, additional_flags: {additional_flags}, check_value: {check_value}");

                    // match self.pixel_type {
                    //     PixelType::TrueColor => {
                    //         let width = self.format.width as usize;
                    //         let samples_per_pixel = 3;
                    //         let bits_per_sample = self.format.bit_depth as usize;
                    //         let scan_line_length = width * samples_per_pixel * bits_per_sample / 8;

                    //         println!("scan_line_length: {scan_line_length}");

                    //         for _ in 0..self.format.height {
                    //             bytes.append(&mut chunk.reader.read_bytes(scan_line_length)?);
                    //         }
                    //     }
                    //     PixelType::GrayScale => {}
                    //     PixelType::IndexedColor => {}
                    // }
                }
                ChunkType::IEND => {
                    println!("yay");
                }
                _ => {
                    println!("todo");
                }
            }
        }

        Ok(bytes)
    }
}

#[derive(Debug)]
struct PNGFormat {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8,
}

impl PNGFormat {
    pub fn from_ihdr_chunk(mut chunk: Chunk) -> Result<Self, Error> {
        debug_assert!(chunk.chunk_type == ChunkType::IHDR);

        let width = chunk.reader.read_u32()?;
        let height = chunk.reader.read_u32()?;
        let bit_depth = chunk.reader.read_u8()?;
        let color_type = chunk.reader.read_u8()?;
        let compression_method = chunk.reader.read_u8()?;
        let filter_method = chunk.reader.read_u8()?;
        let interlace_method = chunk.reader.read_u8()?;

        Ok(Self {
            width,
            height,
            bit_depth,
            color_type,
            compression_method,
            filter_method,
            interlace_method,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
enum ChunkType {
    // Critical (Ordered) chunks
    IHDR,
    // Optional
    PLTE,
    IDAT,
    IEND,

    // Ancillary (Optional, Unordered) chunks
    cHRM,
    gAMA,
    iCCP,
    sBIT,
    sRGB,
    bkGD,
    hIST,
    tRNS,
    pHYs,
    sPLT,
    tIME,
    iTXt,
    tEXt,
    zTXt,
}

struct Chunk {
    chunk_type: ChunkType,
    length: u32,
    reader: ChunkReader,
}

impl Chunk {
    pub fn new(file_reader: &mut Reader<std::fs::File>) -> Result<Self, Error> {
        let length = file_reader.read_u32()?;
        let chunk_type = file_reader.read_bytes(4)?;
        let data = file_reader.read_bytes(length as usize)?;
        let reader = Reader::new(std::io::BufReader::new(std::io::Cursor::new(data)));
        let _crc = file_reader.read_u32()?;

        let chunk_type = match chunk_type
            .iter()
            .map(|n| *n as char)
            .collect::<String>()
            .as_str()
        {
            "IHDR" => ChunkType::IHDR,
            "PLTE" => ChunkType::PLTE,
            "IDAT" => ChunkType::IDAT,
            "IEND" => ChunkType::IEND,
            "cHRM" => ChunkType::cHRM,
            "gAMA" => ChunkType::gAMA,
            "iCCP" => ChunkType::iCCP,
            "sBIT" => ChunkType::sBIT,
            "sRGB" => ChunkType::sRGB,
            "bkGD" => ChunkType::bkGD,
            "hIST" => ChunkType::hIST,
            "tRNS" => ChunkType::tRNS,
            "pHYs" => ChunkType::pHYs,
            "sPLT" => ChunkType::sPLT,
            "tIME" => ChunkType::tIME,
            "iTXt" => ChunkType::iTXt,
            "tEXt" => ChunkType::tEXt,
            "zTXt" => ChunkType::zTXt,
            _ => {
                logger::error!(
                    "Could not determine chunk type: {}",
                    chunk_type.iter().map(|n| *n as char).collect::<String>()
                );

                // TODO: remove
                println!(
                    "Could not determine chunk type: {}",
                    chunk_type.iter().map(|n| *n as char).collect::<String>()
                );
                return Err(Error::UnknownChunkType);
            }
        };

        Ok(Self {
            chunk_type,
            length,
            reader,
        })
    }
}

type ChunkReader = Reader<std::io::Cursor<Vec<u8>>>;

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("length", &self.length)
            .field("chunk_type", &self.chunk_type)
            // .field("crc", &self.crc)
            .finish()
    }
}

struct Reader<T: std::io::Read> {
    buf_reader: std::io::BufReader<T>,
}

impl<T: std::io::Read> Reader<T> {
    pub fn new(buf_reader: std::io::BufReader<T>) -> Self {
        Self { buf_reader }
    }

    pub fn read_bytes(&mut self, bytes_to_read: usize) -> Result<Vec<u8>, Error> {
        if bytes_to_read == 0 {
            return Ok(vec![]);
        }

        let mut buf = vec![0; bytes_to_read];
        self.buf_reader.read_exact(&mut buf).map_err(|err| {
            if err.kind() == std::io::ErrorKind::UnexpectedEof {
                Error::EndOfFile
            } else {
                logger::error!("Failed to read exact: {}", err);
                Error::Reader
            }
        })?;

        Ok(buf)
    }

    pub fn read_u32(&mut self) -> Result<u32, Error> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u16(&mut self) -> Result<u16, Error> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u8(&mut self) -> Result<u8, Error> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0])
    }

    pub fn read_remaining_bytes(&mut self, buf_capacity: usize) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::with_capacity(buf_capacity);
        self.buf_reader
            .read_to_end(&mut buf)
            .map_err(|_| Error::Reader)?;

        Ok(buf)
    }

    pub fn read_null_terminated_string(&mut self) -> Result<String, Error> {
        // TODO: check if this is max
        let mut buf = Vec::with_capacity(79);
        loop {
            let byte = self.read_bytes(1)?;
            if byte[0] == b'\0' {
                break;
            }

            buf.push(byte[0] as char);
        }
        Ok(buf.iter().collect())
    }
}

#[allow(dead_code)]
fn to_bytes<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, Error> {
    let f = std::fs::File::open(path).map_err(|err| {
        logger::error!("Could not open file: {}", err);
        Error::InvalidPath
    })?;
    let buf_reader = std::io::BufReader::new(f);
    let mut reader = Reader::new(buf_reader);

    let png_signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let read_signature = reader.read_bytes(8)?;

    println!("PNG Header: {:?}", read_signature);
    if png_signature != *read_signature {
        logger::error!("Provided file is not a png");
        return Err(Error::InvalidFile);
    }

    let parser = PNGParser::new(reader)?;
    println!("{:#?}", parser);
    parser.parse_to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let _bytes = to_bytes("../../../res/sandbox.png").unwrap();

        panic!();
    }
}
