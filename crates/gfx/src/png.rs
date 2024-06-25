// TODO: remove
#![allow(dead_code)]
use std::{fs::File, io::Read};

use asset::reader::ByteReader;
use libflate::zlib::Decoder;
use logger::{error, warn};

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

// #[derive(Debug, Clone, Copy)]
// struct HuffmanEntry {
//     symbol: u16,
//     bits_used: u16,
// }
//
// impl HuffmanEntry {
//     pub fn new(symbol: u16, bits_used: u16) -> Self {
//         Self { symbol, bits_used }
//     }
// }
//
// struct HuffmanTable {
//     entries: Vec<HuffmanEntry>,
//     // in bits
//     max_code_len: u16,
// }
//
// impl HuffmanTable {
//     // pub fn new(
//     //     max_code_len: u16,
//     //     symbol_count: usize,
//     //     symbol_code_len: Vec<u16>,
//     //     symbol_addend: u16,
//     // ) -> Self {
//     //     let mut entries = Vec::new();
//     //     let mut symbol_table = Vec::new();
//
//     //     let mut code_len_hist = vec![16; 0];
//     //     for i in 0..symbol_count {
//     //         code_len_hist[symbol_code_len[i] as usize] += 1;
//     //     }
//
//     //     let mut next_unused_code = vec![16; 0];
//     //     let mut code = 0;
//     //     code_len_hist[0] = 0;
//     //     for i in 1..next_unused_code.len() {
//     //         code = (code + code_len_hist[i - 1]) << 1;
//     //         next_unused_code[i] = code;
//     //     }
//
//     //     for i in 0..symbol_count {
//     //         let code_len_in_bits = symbol_code_len[i];
//     //         if code_len_in_bits == 0 {
//     //             continue;
//     //         }
//     //         debug_assert!((code_len_in_bits as usize) < next_unused_code.len());
//     //         let code = next_unused_code[code_len_in_bits as usize];
//     //         next_unused_code[code_len_in_bits as usize] += 1;
//
//     //         let entry = HuffmanEntry::new(i as u16 + symbol_addend, code_len_in_bits);
//
//     //         let trash_bits = max_code_len - code_len_in_bits;
//     //         for i in 0..(1 << trash_bits) {
//     //             let index = (i << code_len_in_bits) | code;
//     //             entries[index] = entry;
//     //         }
//     //     }
//
//     //     Self {
//     //         entries,
//     //         max_code_len,
//     //     }
//     // }
//
//     // pub fn decode_next_symbol(&self, bit_reader: &mut BitReader) -> Result<u16, Error> {
//     //     let entry_index = bit_reader.peek_bits_le(self.max_code_len as usize)?;
//     //     let table_len = self.entries.len() as u32;
//     //     debug_assert!(entry_index < table_len);
//
//     //     let entry = &self.entries[entry_index as usize];
//     //     bit_reader.discard_bits(entry.bits_used as usize);
//
//     //     Ok(entry.symbol)
//     // }
// }

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

        let mut chunks: Vec<Chunk> = Vec::new();
        loop {
            if let Some(c) = chunks.last() {
                if c.chunk_type == ChunkType::IEND {
                    break;
                }
            }

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

                    let compressed_data = chunk
                        .reader
                        .read_bytes(chunk.length as usize)
                        .map_err(|_| Error::Reader)?;
                    let mut decoder = Decoder::new(compressed_data.as_slice()).map_err(|err| {
                        error!("{err}");
                        Error::Decoding
                    })?;
                    let mut decoded_data = Vec::new();
                    decoder.read_to_end(&mut decoded_data).map_err(|err| {
                        error!("{err}");
                        Error::Decoding
                    })?;

                    // let decoded_data = Vec::new();
                    // let mut bit_reader = BitReader::new(chunk.reader);

                    // // ZLib Header

                    // let cm = bit_reader.read_bits_to_u8_be(4)?;
                    // if cm != 8 {
                    //     return Err(Error::UnsupportedEncoding);
                    // }
                    // let cinfo = bit_reader.read_bits_to_u8_be(4)?;
                    // let fcheck = bit_reader.read_bits_to_u8_be(5)?;
                    // let fdict = bit_reader.read_bits_to_u8_be(1)?;
                    // if fdict != 0 {
                    //     return Err(Error::UnsupportedEncoding);
                    // }
                    // let flevel = bit_reader.read_bits_to_u8_be(2)?;

                    // // Inflate Data

                    // // let window = Vec::with_capacity(32768);

                    // loop {
                    //     let b_final = bit_reader.read_bit_be()?;
                    //     let block_type = bit_reader.read_bits_to_u8_be(2)?;

                    //     println!("BFinal: {b_final}, Block Type: {block_type:#b}");

                    //     match block_type {
                    //         // No compression
                    //         0b00 => {
                    //             let _ = bit_reader.flush_byte();
                    //             let len = bit_reader.read_u16_be()?;
                    //             let nlen = bit_reader.read_u16_be()?;
                    //             // let block_data = bit_reader.read_bytes(len as usize)?;
                    //             // println!("Block data: {block_data:?}");
                    //             println!("{len}");
                    //             unimplemented!()
                    //         }
                    //         // Compressed with fixed Huffman codes
                    //         0b01 => {
                    //             unimplemented!();
                    //         }
                    //         // Compressed with dynamic Huffman codes
                    //         0b10 => {
                    //             // Construct Huffman Encodings

                    //             let mut hlit_len = bit_reader.read_bits_to_u8_be(5)? as usize;
                    //             let mut hdist_len = bit_reader.read_bits_to_u8_be(5)? as usize;
                    //             let mut hclen_len = bit_reader.read_bits_to_u8_be(4)? as usize;

                    //             hlit_len += 257;
                    //             hdist_len += 1;
                    //             hclen_len += 4;

                    //             let mut hclen = Vec::with_capacity(20);

                    //             let hclen_lookup = [
                    //                 16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1,
                    //                 15,
                    //             ];

                    //             for _ in 0..hclen_len {
                    //                 hclen.push(
                    //                     hclen_lookup[bit_reader.read_bits_to_u8_be(3)? as usize],
                    //                 );
                    //             }

                    //             let huff_dict = HuffmanTable::new(hclen_lookup.len(), hclen);

                    //             println!("{hlit_len}, {hdist_len}, {hclen_len} => {hclen:?}");

                    //             loop {
                    //                 let mut end_block_code = true;

                    //                 // if value < 256 {
                    //                 // } else {
                    //                 //     if value == 256 {
                    //                 //         end_block_code = true;
                    //                 //     } else {

                    //                 //         let distance = ;

                    //                 //     }
                    //                 // }

                    //                 if end_block_code {
                    //                     break;
                    //                 }
                    //             }
                    //         }
                    //         0b11 => return Err(Error::ReservedBlockEncoding),
                    //         _ => unreachable!(),
                    //     }

                    //     if b_final {
                    //         break;
                    //     }
                    // }

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

        let width = chunk.reader.read_u32_be().map_err(|_| Error::Reader)?;
        let height = chunk.reader.read_u32_be().map_err(|_| Error::Reader)?;
        let bit_depth = chunk.reader.read_u8().map_err(|_| Error::Reader)?;
        let color_type = chunk.reader.read_u8().map_err(|_| Error::Reader)?;
        let compression_method = chunk.reader.read_u8().map_err(|_| Error::Reader)?;
        let filter_method = chunk.reader.read_u8().map_err(|_| Error::Reader)?;
        let interlace_method = chunk.reader.read_u8().map_err(|_| Error::Reader)?;

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
        let length = file_reader.read_u32_be().map_err(|_| Error::Reader)?;
        let chunk_type = file_reader.read_bytes(4).map_err(|_| Error::Reader)?;
        let data = file_reader
            .read_bytes(length as usize)
            .map_err(|_| Error::Reader)?;
        let reader = ByteReader::new(std::io::BufReader::new(std::io::Cursor::new(data)));
        let _crc = file_reader.read_u32_be().map_err(|_| Error::Reader)?;

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

// struct Inflater {
//     bit_reader: BitReader,
//     comp_window: usize,
//     cinf: u8,
//     cm: u8,
//     fcheck: u8,
//     fdict: u8,
//     flevel: u8,
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endian {
    BE,
    LE,
}

// struct BitReader {
//     byte_reader: ChunkByteReader,
//     byte_buf: VecDeque<(u8, Endian)>,
//     offset: usize,
// }
//
// impl BitReader {
//     pub fn new(byte_reader: ChunkByteReader) -> Self {
//         Self {
//             byte_reader,
//             byte_buf: VecDeque::new(),
//             offset: 0,
//         }
//     }
//
//     pub fn flush_byte(&mut self) {
//         self.byte_buf.pop_front();
//     }
//
//     pub fn discard_bits(&mut self, num_bits: usize) {
//         let _ = self.read_bits_le(num_bits);
//     }
//
//     // pub fn read_bit_le(&mut self) -> Result<bool, Error> {
//     //     let last_byte = if let Some((last_byte, endian)) = self.byte_buf.front() {
//     //         match endian {
//     //             Endian::LE => *last_byte,
//     //             Endian::BE => {
//     //                 return Err(Error::Decoding);
//     //             }
//     //         }
//     //     } else {
//     //         self.byte_buf
//     //             .push_back((self.byte_reader.read_u8()?, Endian::LE));
//     //         self.offset = 7;
//     //         self.byte_buf.back().unwrap().0
//     //     };
//
//     //     let next_bit = match last_byte >> self.offset & 0b0001 {
//     //         0b001 => true,
//     //         0b000 => false,
//     //         _ => unreachable!(),
//     //     };
//
//     //     if self.offset == 0 {
//     //         self.byte_buf.pop_front();
//     //     } else {
//     //         self.offset -= 1;
//     //     }
//
//     //     Ok(next_bit)
//     // }
//
//     pub fn read_bit_be(&mut self) -> Result<bool, Error> {
//         let last_byte = if let Some((last_byte, endian)) = self.byte_buf.front() {
//             match endian {
//                 Endian::BE => *last_byte,
//                 Endian::LE => {
//                     return Err(Error::Decoding);
//                 }
//             }
//         } else {
//             self.byte_buf.push_back((
//                 self.byte_reader.read_u8().map_err(|_| Error::Reader)?,
//                 Endian::BE,
//             ));
//             self.offset = 0;
//             self.byte_buf.back().unwrap().0
//         };
//
//         let next_bit = match last_byte >> self.offset & 0b0001 {
//             0b001 => true,
//             0b000 => false,
//             _ => unreachable!(),
//         };
//
//         if self.offset == 7 {
//             self.byte_buf.pop_front();
//         } else {
//             self.offset += 1;
//         }
//
//         Ok(next_bit)
//     }
//
//     pub fn peek_bit_be(&mut self, bit_index: usize) -> Result<bool, Error> {
//         let byte_depth = bit_index / 8;
//
//         while self.byte_buf.get(byte_depth).is_none() {
//             self.byte_buf.push_back((
//                 self.byte_reader.read_u8().map_err(|_| Error::Reader)?,
//                 Endian::BE,
//             ));
//         }
//
//         if self.byte_buf[byte_depth].1 == Endian::LE {
//             return Err(Error::Decoding);
//         }
//
//         let next_bit = match self.byte_buf[byte_depth].0 >> (bit_index as u8 % 8) & 0b0001 {
//             0b001 => true,
//             0b000 => false,
//             _ => unreachable!(),
//         };
//
//         Ok(next_bit)
//     }
//
//     // pub fn read_bits_le(&mut self, num_bits: usize) -> Result<Vec<bool>, Error> {
//     //     let mut bits = Vec::with_capacity(num_bits);
//     //     for _ in 0..num_bits {
//     //         bits.push(self.read_bit_le()?);
//     //     }
//
//     //     Ok(bits)
//     // }
//
//     pub fn read_bits_be(&mut self, num_bits: usize) -> Result<Vec<bool>, Error> {
//         let mut bits = Vec::with_capacity(num_bits);
//         for _ in 0..num_bits {
//             bits.push(self.read_bit_be()?);
//         }
//
//         bits.reverse();
//
//         Ok(bits)
//     }
//
//     pub fn peek_bits_be(&mut self, num_bits: usize) -> Result<u32, Error> {
//         let mut bits = Vec::with_capacity(num_bits);
//         for i in 0..num_bits {
//             bits.push(self.peek_bit_be(i)?);
//         }
//
//         bits.reverse();
//
//         let mut final_uint = 0;
//         for bit in bits.into_iter() {
//             final_uint <<= 1;
//             if bit {
//                 final_uint |= 1;
//             }
//         }
//
//         Ok(final_uint)
//     }
//
//     // pub fn read_bits_to_u8_le(&mut self, num_bits: usize) -> Result<u8, Error> {
//     //     let mut bits = 0;
//     //     for bit in self.read_bits_le(num_bits)?.into_iter() {
//     //         bits <<= 1;
//     //         if bit {
//     //             bits |= 1;
//     //         }
//     //     }
//
//     //     Ok(bits)
//     // }
//
//     pub fn read_bits_to_u8_be(&mut self, num_bits: usize) -> Result<u8, Error> {
//         let mut bits = 0;
//         for bit in self.read_bits_be(num_bits)?.into_iter() {
//             bits <<= 1;
//             if bit {
//                 bits |= 1;
//             }
//         }
//
//         Ok(bits)
//     }
//
//     pub fn read_u16_be(&mut self) -> Result<u16, Error> {
//         let b1 = self.read_bits_to_u8_be(4)?;
//         let b2 = self.read_bits_to_u8_be(4)?;
//         Ok(u16::from_be_bytes([b2, b1]))
//     }
// }

pub fn to_bytes(mut reader: ByteReader<File>) -> Result<(Vec<u8>, (u32, u32)), Error> {
    let png_signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let read_signature = reader.read_bytes(8).map_err(|_| Error::Reader)?;

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
        let res = bit_reader.read_bits_to_u8_be(1).unwrap();
        println!("{0:#b}", res);
        let res = bit_reader.read_bits_to_u8_be(5).unwrap();
        println!("{0:#b}", res);

        panic!();
    }
}
