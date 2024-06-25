use std::io::{BufReader, Read};

pub enum Error {
    EndOfBuf,
    Interrupted,
}

pub struct ByteReader<R: Read> {
    buf_reader: BufReader<R>,
}

impl<R: Read> ByteReader<R> {
    pub fn new(buf_reader: BufReader<R>) -> Self {
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
                Error::Interrupted
            }
        })?;

        Ok(buf)
    }

    pub fn read_u32_le(&mut self) -> Result<u32, Error> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u16_le(&mut self) -> Result<u16, Error> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_i16_le(&mut self) -> Result<i16, Error> {
        let bytes = self.read_bytes(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_string_le(&mut self, len: usize) -> Result<String, Error> {
        let buf = self.read_bytes(len)?;
        Ok(buf.iter().map(|b| *b as char).collect())
    }

    pub fn read_u8(&mut self) -> Result<u8, Error> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0])
    }

    pub fn read_u32_be(&mut self) -> Result<u32, Error> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u16_be(&mut self) -> Result<u16, Error> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
}
