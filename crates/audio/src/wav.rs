use std::{fs::File, io::Cursor};

use asset::reader::ByteReader;
use logger::warn;

impl From<asset::reader::Error> for Error {
    fn from(_value: asset::reader::Error) -> Self {
        Error::Reader
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Reader,
    InvalidFile,
    InvalidPath,
    UnknownChunkId,
    EndOfBuf,
}

pub fn load_from_bytes(mut reader: ByteReader<File>) -> Result<(Vec<u8>, WavFormat), Error> {
    let parser = WavParser::new(&mut reader)?;
    Ok(parser.parse_to_bytes()?)
}

#[derive(Debug)]
struct WavParser {
    format: WavFormat,
    chunks: Vec<Chunk>,
}

impl WavParser {
    pub fn new(reader: &mut ByteReader<File>) -> Result<Self, Error> {
        parse_wav_header(reader)?;
        let fmt_chunk = Chunk::new(reader)?;
        let format = WavFormat::from_fmt_chunk(fmt_chunk)?;

        let mut chunks = Vec::new();
        loop {
            let next_chunk = Chunk::new(reader);
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

        Ok(Self { format, chunks })
    }

    pub fn parse_to_bytes(self) -> Result<(Vec<u8>, WavFormat), Error> {
        // let total_bytes = self.format.width * self.format.height * 4;
        // let mut bytes = Vec::with_capacity(total_bytes as usize);
        let mut bytes = Vec::new();

        println!("{self:#?}");

        for mut chunk in self.chunks.into_iter() {
            match chunk.chunk_id {
                ChunkId::data => {
                    let pad_byte = chunk.length % 2 != 0;
                    let data_len = if pad_byte {
                        chunk.length - 1
                    } else {
                        chunk.length
                    };

                    match self.format.format_code {
                        FormatCode::PCM => {
                            if self.format.bits_per_sample <= 8 {
                                unimplemented!();
                            }

                            let bytes_per_sample =
                                self.format.block_align as u32 / self.format.channels as u32;
                            let total_samples = data_len / bytes_per_sample as u32;
                            bytes.reserve_exact((bytes_per_sample * total_samples) as usize);

                            for _ in 0..total_samples {
                                let mut sample =
                                    chunk.reader.read_bytes(bytes_per_sample as usize)?;
                                bytes.append(&mut sample);
                            }
                        }
                        _ => unimplemented!(),
                    }
                }

                _ => {
                    println!("Did not parse chunk in PNG: {:?}", chunk.chunk_id);
                    warn!("Did not parse chunk in PNG: {:?}", chunk.chunk_id);
                }
            }
        }

        Ok((bytes, self.format))
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct ExtendedWavFormat {
    pub valid_bits_per_sample: u16,
    pub channel_mask: u32,
    pub sub_format: Vec<u8>,
}

#[derive(Debug)]
#[allow(unused)]
pub struct WavFormat {
    pub format_code: FormatCode,
    pub channels: u16,
    pub samples_per_sec: u32,
    pub avg_bytes_per_sec: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub extended_format: Option<ExtendedWavFormat>,
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum FormatCode {
    PCM,
    IEEE_FLOAT,
    ALAW,
    MULAW,
    EXTENSIBLE,
}

impl WavFormat {
    pub fn from_fmt_chunk(mut chunk: Chunk) -> Result<Self, Error> {
        debug_assert!(chunk.chunk_id == ChunkId::fmt);

        let format_tag = chunk.reader.read_u16_le()?;
        let format_code = match format_tag {
            0x0001 => FormatCode::PCM,
            0x0003 => FormatCode::IEEE_FLOAT,
            0x0006 => FormatCode::ALAW,
            0x0007 => FormatCode::MULAW,
            0xFFFE => FormatCode::EXTENSIBLE,
            _ => unreachable!(),
        };

        let channels = chunk.reader.read_u16_le()?;
        let samples_per_sec = chunk.reader.read_u32_le()?;
        let avg_bytes_per_sec = chunk.reader.read_u32_le()?;
        let block_align = chunk.reader.read_u16_le()?;
        let bits_per_sample = chunk.reader.read_u16_le()?;
        let extended_format = if chunk.length == 18 || chunk.length == 40 {
            let ext_size = chunk.reader.read_u16_le()?;
            let extended_format = if ext_size == 22 {
                let valid_bits_per_sample = chunk.reader.read_u16_le()?;
                let channel_mask = chunk.reader.read_u32_le()?;
                let sub_format = chunk.reader.read_bytes(16)?;
                Some(ExtendedWavFormat {
                    valid_bits_per_sample,
                    channel_mask,
                    sub_format,
                })
            } else {
                None
            };

            extended_format
        } else {
            None
        };

        Ok(Self {
            format_code,
            channels,
            samples_per_sec,
            avg_bytes_per_sec,
            block_align,
            bits_per_sample,
            extended_format,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
enum ChunkId {
    // Master chunk
    RIFF,
    // Sub chunks
    LIST,
    fmt,
    data,
}

pub struct Chunk {
    chunk_id: ChunkId,
    length: u32,
    reader: ByteReader<Cursor<Vec<u8>>>,
}

impl Chunk {
    pub fn new(file_reader: &mut ByteReader<File>) -> Result<Self, Error> {
        let chunk_id = file_reader.read_string_le(4)?;
        let length = file_reader.read_u32_le()?;
        let data = file_reader.read_bytes(length as usize)?;
        let reader = ByteReader::new(std::io::BufReader::new(std::io::Cursor::new(data)));

        let chunk_id = match chunk_id.as_str() {
            "RIFF" => ChunkId::RIFF,
            "LIST" => ChunkId::LIST,
            "fmt " => ChunkId::fmt,
            "data" => ChunkId::data,
            _ => {
                logger::error!("Could not determine chunk id: {}", chunk_id);
                println!("Could not determine chunk id: {}", chunk_id);
                return Err(Error::UnknownChunkId);
            }
        };

        Ok(Self {
            chunk_id,
            length,
            reader,
        })
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("length", &self.length)
            .field("chunk_id", &self.chunk_id)
            // .field("crc", &self.crc)
            .finish()
    }
}

fn parse_wav_header(reader: &mut ByteReader<File>) -> Result<(), Error> {
    let _id = reader.read_string_le(4)?;
    let _size = reader.read_u32_le()?;
    let _wave_id = reader.read_string_le(4)?;

    Ok(())
}
