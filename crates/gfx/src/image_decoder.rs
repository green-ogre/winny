// TODO: remove
#![allow(dead_code)]
use std::{fmt::write, io::Read, path::Path};

use libflate::zlib::Decoder;
use logger::{debug, error, info, warn};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Reader,
    InvalidFile,
    InvalidPath,
    UnknownChunkType,
    EndOfBuf,
    ReservedBlockEncoding,
    Decoding,
    UnsupportedEncoding,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::Reader => "Reader could not read exact bytes from buffer",
            Self::InvalidFile => "The given file is not a PNG",
            Self::InvalidPath => "There is no such file at the given path",
            Self::UnknownChunkType => "This PNG format is unimplemented",
            Self::EndOfBuf => "Reader has overrun the inner buffer",
            Self::ReservedBlockEncoding => "Decoding error, this is a bug",
            Self::Decoding => "Failed to decode data, this is a bug",
            Self::UnsupportedEncoding => "File used an unknown form of compression",
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
    pub fn new(mut reader: ByteReader<std::fs::File>) -> Result<Self, Error> {
        // TODO: support different formats

        let ihdr_chunk = Chunk::new(&mut reader)?;
        let format = PNGFormat::from_ihdr_chunk(ihdr_chunk)?;

        let mut chunks = Vec::new();
        loop {
            let next_chunk = Chunk::new(&mut reader);
            match next_chunk {
                Ok(next_chunk) => chunks.push(next_chunk),
                Err(err) => {
                    if err == Error::EndOfBuf {
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
        let total_bytes = self.format.width * self.format.height * 4;
        let mut bytes = Vec::with_capacity(total_bytes as usize);

        for mut chunk in self.chunks.into_iter() {
            match chunk.chunk_type {
                // ChunkType::PLTE => {
                //     let palette = chunk.reader.read_bytes(chunk.length as usize * 3)?;
                //     info!("palette: {palette:?}");
                // }
                // ChunkType::pHYs => {
                //     let ppux = chunk.reader.read_u32()?;
                //     let ppuy = chunk.reader.read_u32()?;
                //     let unit = chunk.reader.read_u8()?;
                //     info!("ppux: {ppux}, ppuy: {ppuy}, unit: {unit}");
                // }
                // ChunkType::iCCP => {
                //     let profile_name = chunk.reader.read_null_terminated_string()?;
                //     let compression_method = chunk.reader.read_u8()?;

                //     let uncompressed_profile = match compression_method {
                //         0 => {
                //             chunk
                //                 .reader
                //                 .read_string(chunk.length as usize - 1 - profile_name.len() - 1)?;
                //         }
                //         _ => {
                //             chunk.reader.read_remaining_bytes(chunk.length as usize)?;
                //         }
                //     };

                //     info!("profile_name: {profile_name:?}, compression_method: {compression_method}, compressed_profile: {uncompressed_profile:?}");
                // }
                ChunkType::tIME => {
                    //     let year = chunk.reader.read_u16()?;
                    //     let month = chunk.reader.read_u8()?;
                    //     let day = chunk.reader.read_u8()?;
                    //     let hour = chunk.reader.read_u8()?;
                    //     let minute = chunk.reader.read_u8()?;
                    //     let second = chunk.reader.read_u8()?;
                    //     info!("year: {year}, month: {month}, day: {day}, hour: {hour}, minute: {minute}, second: {second}");
                }
                // ChunkType::tEXt => {
                //     let keyword = chunk.reader.read_null_terminated_string()?;
                //     let text = chunk.reader.read_remaining_bytes(chunk.length as usize)?;
                //     let text = text.into_iter().map(|b| b as char).collect::<String>();
                //     info!("keyword: {keyword}, text: {text}")
                // }
                ChunkType::IDAT => {
                    // 1. Inflate the data using zlib decoder

                    // let compressed_data = chunk.reader.read_bytes(chunk.length as usize)?;
                    // let mut decoder = Decoder::new(compressed_data.as_slice()).map_err(|err| {
                    //     error!("{err}");
                    //     Error::Decoding
                    // })?;
                    // let mut decoded_data = Vec::new();
                    // decoder.read_to_end(&mut decoded_data).map_err(|err| {
                    //     error!("{err}");
                    //     Error::Decoding
                    // })?;

                    // let cm = chunk.reader.read_u8()?;

                    let decoded_data = Vec::new();
                    let mut bit_reader = BitReader::new(chunk.reader);

                    // ZLib Header

                    let cm = bit_reader.read_bits_to_u8_be(4)?;
                    if cm != 8 {
                        return Err(Error::UnsupportedEncoding);
                    }
                    let cinfo = bit_reader.read_bits_to_u8_be(4)?;
                    let fcheck = bit_reader.read_bits_to_u8_be(5)?;
                    let fdict = bit_reader.read_bits_to_u8_be(1)?;
                    if fdict != 0 {
                        return Err(Error::UnsupportedEncoding);
                    }
                    let flevel = bit_reader.read_bits_to_u8_be(2)?;

                    // Inflate Data

                    // let window = Vec::with_capacity(32768);

                    loop {
                        let b_final = bit_reader.read_bit_be()?;
                        let block_type = bit_reader.read_bits_to_u8_be(2)?;

                        match block_type {
                            // No compression
                            0b00 => {
                                let _ = bit_reader.flush_byte();
                                let len = bit_reader.read_u16_be()?;
                                let nlen = bit_reader.read_u16_be()?;
                                // let block_data = bit_reader.read_bytes(len as usize)?;
                                // println!("Block data: {block_data:?}");
                                println!("{len}");
                                unimplemented!()
                            }
                            // Compressed with fixed Huffman codes
                            0b01 => {
                                unimplemented!();
                            }
                            // Compressed with dynamic Huffman codes
                            0b10 => {
                                // Construct Huffman Encodings

                                let mut hlit_len = bit_reader.read_bits_to_u8_be(5)? as usize;
                                let mut hdist_len = bit_reader.read_bits_to_u8_be(5)? as usize;
                                let mut hclen_len = bit_reader.read_bits_to_u8_be(4)? as usize;

                                hlit_len += 257;
                                hdist_len += 1;
                                hclen_len += 4;

                                let mut hclen = Vec::with_capacity(20);

                                let hclen_lookup = [
                                    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1,
                                    15,
                                ];

                                for _ in 0..hclen_len {
                                    hclen.push(
                                        hclen_lookup[bit_reader.read_bits_to_u8_be(3)? as usize],
                                    );
                                }

                                println!("{hlit_len}, {hdist_len}, {hclen_len} => {hclen:?}");

                                loop {
                                    let mut end_block_code = true;

                                    // if value < 256 {
                                    // } else {
                                    //     if value == 256 {
                                    //         end_block_code = true;
                                    //     } else {

                                    //         let distance = ;

                                    //     }
                                    // }

                                    if end_block_code {
                                        break;
                                    }
                                }
                            }
                            0b11 => return Err(Error::ReservedBlockEncoding),
                            _ => unreachable!(),
                        }

                        if b_final {
                            break;
                        }
                    }

                    // 2. Reverse the filtering applied to scanlines

                    let mut prev_decoded_scanline = Vec::with_capacity(
                        self.format.width as usize * self.format.bytes_per_pixel,
                    );

                    let scanline_len = self.format.width as usize * self.format.bytes_per_pixel + 1;
                    let decoded_scanline_len = self.format.width as usize * 4;

                    for offset in 0..self.format.height as usize {
                        let offset = offset * scanline_len;
                        let filter_type = &decoded_data[offset..offset + 1];
                        let scanline = &decoded_data[offset + 1..offset + scanline_len];

                        let mut decoded_scanline: Vec<u8> =
                            Vec::with_capacity(decoded_scanline_len);

                        let mut pixel: Vec<u8> = Vec::with_capacity(4);
                        for (index, byte) in scanline.iter().enumerate() {
                            let byte = match filter_type[0] {
                                // None
                                0 => *byte,

                                // Sub
                                1 => {
                                    // Beggining of file
                                    if index < self.format.bytes_per_pixel {
                                        scanline[index]
                                    } else {
                                        ((*byte as u16
                                            + decoded_scanline[index - self.format.bytes_per_pixel]
                                                as u16)
                                            % 256) as u8
                                    }
                                }

                                // Up
                                2 => {
                                    ((*byte as u16 + prev_decoded_scanline[index] as u16) % 256)
                                        as u8
                                }

                                // Average
                                3 => unimplemented!(),

                                // Paeth
                                4 => {
                                    let left = if index < self.format.bytes_per_pixel {
                                        0
                                    } else {
                                        decoded_scanline[index - self.format.bytes_per_pixel]
                                    };

                                    let up = if prev_decoded_scanline.len() == 0 {
                                        0
                                    } else {
                                        prev_decoded_scanline[index]
                                    };

                                    let top_left = if prev_decoded_scanline.len() == 0 {
                                        0
                                    } else if index < self.format.bytes_per_pixel {
                                        0
                                    } else {
                                        prev_decoded_scanline[index - self.format.bytes_per_pixel]
                                    };

                                    ((*byte as u16
                                        + paeth_predictor(left as i32, up as i32, top_left as i32)
                                            as u16)
                                        % 256) as u8
                                }

                                _ => unreachable!(),
                            };

                            decoded_scanline.push(byte);
                            pixel.push(byte);

                            if self.format.bytes_per_pixel == 3 {
                                if pixel.len() == 3 {
                                    pixel.push(0xff);
                                    bytes.append(&mut pixel);
                                }
                            } else {
                                if pixel.len() == 4 {
                                    bytes.append(&mut pixel);
                                }
                            }
                        }

                        prev_decoded_scanline = decoded_scanline.to_vec();
                    }
                }
                ChunkType::IEND => {}
                _ => {
                    warn!("Did not parse chunk in PNG: {:?}", chunk.chunk_type);
                }
            }
        }

        Ok(bytes)
    }
}

fn paeth_predictor(a: i32, b: i32, c: i32) -> u8 {
    let p = a + b - c;
    let pa = p.abs_diff(a);
    let pb = p.abs_diff(b);
    let pc = p.abs_diff(c);

    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
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
    bytes_per_pixel: usize,
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

        let bytes_per_pixel = match color_type {
            // RGB
            2 => bit_depth as usize / 8 * 3,
            // RGBA
            6 => bit_depth as usize / 8 * 4,
            _ => unimplemented!(),
        };

        Ok(Self {
            width,
            height,
            bit_depth,
            color_type,
            compression_method,
            filter_method,
            interlace_method,
            bytes_per_pixel,
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
    reader: ChunkByteReader,
}

impl Chunk {
    pub fn new(file_reader: &mut ByteReader<std::fs::File>) -> Result<Self, Error> {
        let length = file_reader.read_u32()?;
        let chunk_type = file_reader.read_bytes(4)?;
        let data = file_reader.read_bytes(length as usize)?;
        let reader = ByteReader::new(std::io::BufReader::new(std::io::Cursor::new(data)));
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

type ChunkByteReader = ByteReader<std::io::Cursor<Vec<u8>>>;

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("length", &self.length)
            .field("chunk_type", &self.chunk_type)
            // .field("crc", &self.crc)
            .finish()
    }
}

struct Inflater {
    bit_reader: BitReader,
    comp_window: usize,
    cinf: u8,
    cm: u8,
    fcheck: u8,
    fdict: u8,
    flevel: u8,
}

#[derive(Debug, Clone, Copy)]
enum Endian {
    BE,
    LE,
}

struct BitReader {
    byte_reader: ChunkByteReader,
    last_byte: Option<(u8, Endian)>,
    offset: usize,
}

impl BitReader {
    pub fn new(byte_reader: ChunkByteReader) -> Self {
        Self {
            byte_reader,
            last_byte: None,
            offset: 0,
        }
    }

    pub fn flush_byte(&mut self) {
        self.last_byte = None;
    }

    pub fn read_bit_le(&mut self) -> Result<bool, Error> {
        let last_byte = if let Some((last_byte, endian)) = self.last_byte {
            match endian {
                Endian::LE => last_byte,
                Endian::BE => {
                    return Err(Error::Decoding);
                }
            }
        } else {
            self.last_byte = Some((self.byte_reader.read_u8()?, Endian::LE));
            self.offset = 7;
            self.last_byte.unwrap().0
        };

        let next_bit = match last_byte >> self.offset & 0b0001 {
            0b001 => true,
            0b000 => false,
            _ => unreachable!(),
        };

        if self.offset == 0 {
            self.last_byte = None;
        } else {
            self.offset -= 1;
        }

        Ok(next_bit)
    }

    pub fn read_bit_be(&mut self) -> Result<bool, Error> {
        let last_byte = if let Some((last_byte, endian)) = self.last_byte {
            match endian {
                Endian::BE => last_byte,
                Endian::LE => {
                    return Err(Error::Decoding);
                }
            }
        } else {
            self.last_byte = Some((self.byte_reader.read_u8()?, Endian::BE));
            self.offset = 0;
            self.last_byte.unwrap().0
        };

        let next_bit = match last_byte >> self.offset & 0b0001 {
            0b001 => true,
            0b000 => false,
            _ => unreachable!(),
        };

        if self.offset == 7 {
            self.last_byte = None;
        } else {
            self.offset += 1;
        }

        Ok(next_bit)
    }

    pub fn read_bits_le(&mut self, num_bits: usize) -> Result<Vec<bool>, Error> {
        let mut bits = Vec::with_capacity(num_bits);
        for _ in 0..num_bits {
            bits.push(self.read_bit_le()?);
        }

        Ok(bits)
    }

    pub fn read_bits_be(&mut self, num_bits: usize) -> Result<Vec<bool>, Error> {
        let mut bits = Vec::with_capacity(num_bits);
        for _ in 0..num_bits {
            bits.push(self.read_bit_be()?);
        }

        bits.reverse();

        Ok(bits)
    }

    pub fn read_bits_to_u8_le(&mut self, num_bits: usize) -> Result<u8, Error> {
        let mut bits = 0;
        for bit in self.read_bits_le(num_bits)?.into_iter() {
            bits <<= 1;
            if bit {
                bits |= 1;
            }
        }

        Ok(bits)
    }

    // TODO: only this one works right
    pub fn read_bits_to_u8_be(&mut self, num_bits: usize) -> Result<u8, Error> {
        let mut bits = 0;
        for bit in self.read_bits_be(num_bits)?.into_iter() {
            bits <<= 1;
            if bit {
                bits |= 1;
            }
        }

        Ok(bits)
    }

    pub fn read_u16_be(&mut self) -> Result<u16, Error> {
        let b1 = self.read_bits_to_u8_be(4)?;
        let b2 = self.read_bits_to_u8_be(4)?;
        Ok(u16::from_be_bytes([b2, b1]))
    }
}

struct ByteReader<T: std::io::Read> {
    buf_reader: std::io::BufReader<T>,
}

impl<T: std::io::Read> ByteReader<T> {
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
                Error::EndOfBuf
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

    pub fn read_string(&mut self, len: usize) -> Result<String, Error> {
        let buf = self.read_bytes(len)?;
        Ok(buf.iter().map(|b| *b as char).collect())
    }
}

pub fn to_bytes<P: AsRef<Path>>(path: P) -> Result<(Vec<u8>, (u32, u32)), Error> {
    let f = std::fs::File::open(path).map_err(|err| {
        logger::error!("Could not open file: {}", err);
        Error::InvalidPath
    })?;
    let buf_reader = std::io::BufReader::new(f);
    let mut reader = ByteReader::new(buf_reader);

    let png_signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let read_signature = reader.read_bytes(8)?;

    if png_signature != *read_signature {
        logger::error!("Provided file is not a png");
        return Err(Error::InvalidFile);
    }

    let parser = PNGParser::new(reader)?;

    let width = parser.format.width;
    let height = parser.format.height;
    let dimensions = (width, height);

    let bytes = parser.parse_to_bytes()?;

    Ok((bytes, dimensions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let _bytes = to_bytes("../../../res/sandbox.png").unwrap();

        panic!();
    }

    #[test]
    fn bits() {
        let num = 0x0f;
        let mut bit_reader = BitReader::new(ByteReader::new(std::io::BufReader::new(
            std::io::Cursor::new(vec![num]),
        )));
        let res = bit_reader.read_bits_to_u8(1).unwrap();
        println!("{0:#b}", res);
        let res = bit_reader.read_bits_to_u8(5).unwrap();
        println!("{0:#b}", res);

        panic!();
    }
}
