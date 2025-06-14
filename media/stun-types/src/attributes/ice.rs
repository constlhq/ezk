use super::Attribute;
use crate::{
    Error, NE,
    builder::MessageBuilder,
    parse::{AttrSpan, Message},
};
use byteorder::ReadBytesExt;
use bytes::BufMut;

pub struct Priority(pub u32);

impl Attribute<'_> for Priority {
    type Context = ();
    const TYPE: u16 = 0x0024;

    fn decode(_: Self::Context, msg: &mut Message, attr: AttrSpan) -> Result<Self, Error> {
        let mut value = attr.get_value(msg.buffer());

        if value.len() != 4 {
            return Err(Error::InvalidData("priority value must be 4 bytes"));
        }

        Ok(Self(value.read_u32::<NE>()?))
    }

    fn encode(&self, _: Self::Context, builder: &mut MessageBuilder) {
        let data = builder.buffer();
        data.put_u32(self.0);
    }

    fn encode_len(&self) -> Result<u16, Error> {
        Ok(4)
    }
}

pub struct UseCandidate;

impl Attribute<'_> for UseCandidate {
    type Context = ();
    const TYPE: u16 = 0x0025;

    fn decode(_: Self::Context, _msg: &mut Message, _attr: AttrSpan) -> Result<Self, Error> {
        Ok(Self)
    }

    fn encode(&self, _: Self::Context, _builder: &mut MessageBuilder) {}

    fn encode_len(&self) -> Result<u16, Error> {
        Ok(0)
    }
}

pub struct IceControlled(pub u64);

impl Attribute<'_> for IceControlled {
    type Context = ();
    const TYPE: u16 = 0x8029;

    fn decode(_: Self::Context, msg: &mut Message, attr: AttrSpan) -> Result<Self, Error> {
        let mut value = attr.get_value(msg.buffer());

        if value.len() != 8 {
            return Err(Error::InvalidData("ice-controlled value must be 8 bytes"));
        }

        Ok(Self(value.read_u64::<NE>()?))
    }

    fn encode(&self, _: Self::Context, builder: &mut MessageBuilder) {
        let data = builder.buffer();
        data.put_u64(self.0);
    }

    fn encode_len(&self) -> Result<u16, Error> {
        Ok(8)
    }
}

pub struct IceControlling(pub u64);

impl Attribute<'_> for IceControlling {
    type Context = ();
    const TYPE: u16 = 0x802A;

    fn decode(_: Self::Context, msg: &mut Message, attr: AttrSpan) -> Result<Self, Error> {
        let mut value = attr.get_value(msg.buffer());

        if value.len() != 8 {
            return Err(Error::InvalidData("ice-controlling value must be 8 bytes"));
        }

        Ok(Self(value.read_u64::<NE>()?))
    }

    fn encode(&self, _: Self::Context, builder: &mut MessageBuilder) {
        let data = builder.buffer();
        data.put_u64(self.0);
    }

    fn encode_len(&self) -> Result<u16, Error> {
        Ok(8)
    }
}
