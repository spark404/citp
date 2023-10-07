use byteorder::LittleEndian;
use protocol::{
    self, ReadBytes, ReadBytesExt, ReadFromBytes, SizeBytes, WriteBytes, WriteBytesExt,
    WriteToBytes, LE,
};
use std::ffi::CString;
use std::{io, mem};

/// The old port originally used for broadcast.
pub const OLD_BROADCAST_PORT: u16 = 4810;

/// The newer port used for multicast.
pub const MULTICAST_PORT: u16 = 4809;

/// The old multicast address prior to early 2014.
pub const OLD_MULTICAST_ADDR: [u8; 4] = [224, 0, 0, 180];

/// The official multicast address since early 2014.
pub const MULTICAST_ADDR: [u8; 4] = [239, 224, 0, 180];

/// The PINF layer provides a standard, single, header used at the start of all PINF packets.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Header {
    /// The CITP header. CITP ContentType is "PINF".
    pub citp_header: protocol::Header,
    /// A cookie defining which PINF message it is.
    pub content_type: u32,
}

/// Layout of PINF messages.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Message<T> {
    /// The PINF header - the base header with the PINF content type.
    pub pinf_header: Header,
    /// The data for the message.
    pub message: T,
}

/// ## PINF / PNam - Peer Name message.
///
/// The PeerName message provides the receiver with a display name of the peer. In early
/// implementations of CITP, the PNam message was broacasted as a means of locating peers - now the
/// PLoc message is multicasted instead. The PNam message is useful though, as a message
/// transferred from a peer connected to a listening peer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PNam {
    /// The display name of the peer (null terminated). This should be anything from a user defined
    /// alias for the peer of the name of the product, or a combination.
    pub name: CString,
}

/// ## PINF / PLoc - Peer Location message.
///
/// The peer location message provides the receiver with connectivity information. If the
/// listeningTCPPort field is non-null, it may be possible to connect to the peer on that port
/// using TCP. If the peer can only handle a limited number of simultaneous connections, then
/// additional connections should be actively refused. The `type` field instructs the receiver what
/// kind of peer it is and the name and state fields provide display name and information.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PLoc {
    /// The port on which the peer is listening for incoming TCP connections. `0` if not listening.
    pub listening_tcp_port: u16,
    /// Can be "LightingConsole", "MediaServer" or "Visualiser".
    pub kind: CString,
    /// The display name of the peer. Corresponds to the `pinf::PNam::name` field.
    pub name: CString,
    /// The display state of the peer. This can be descriptive string presentable to the user such
    /// as "Idle", "Running", etc.
    pub state: CString,
}

impl Header {
    pub const CONTENT_TYPE: &'static [u8] = b"PINF";
}

impl PNam {
    pub const CONTENT_TYPE: &'static [u8] = b"PNam";
}

impl PLoc {
    pub const CONTENT_TYPE: &'static [u8] = b"PLoc";
}

impl WriteToBytes for Header {
    fn write_to_bytes<W: WriteBytesExt>(&self, mut writer: W) -> io::Result<()> {
        writer.write_bytes(&self.citp_header)?;
        writer.write_u32::<LE>(self.content_type)?;
        Ok(())
    }
}

impl<T> WriteToBytes for Message<T>
where
    T: WriteToBytes,
{
    fn write_to_bytes<W: WriteBytesExt>(&self, mut writer: W) -> io::Result<()> {
        writer.write_bytes(&self.pinf_header)?;
        writer.write_bytes(&self.message)?;
        Ok(())
    }
}

impl WriteToBytes for PNam {
    fn write_to_bytes<W: WriteBytesExt>(&self, mut writer: W) -> io::Result<()> {
        writer.write_bytes(&self.name)?;
        Ok(())
    }
}

impl WriteToBytes for PLoc {
    fn write_to_bytes<W: WriteBytesExt>(&self, mut writer: W) -> io::Result<()> {
        writer.write_u16::<LE>(self.listening_tcp_port)?;
        writer.write_bytes(&self.kind)?;
        writer.write_bytes(&self.name)?;
        writer.write_bytes(&self.state)?;
        Ok(())
    }
}

impl ReadFromBytes for Header {
    fn read_from_bytes<R: ReadBytesExt>(mut reader: R) -> io::Result<Self> {
        let header = Header {
            citp_header: reader.read_bytes()?,
            content_type: reader.read_u32::<LittleEndian>()?,
        };
        Ok(header)
    }
}
impl ReadFromBytes for Message<PLoc> {
    fn read_from_bytes<R: ReadBytesExt>(mut reader: R) -> io::Result<Self> {
        let msg = Message::<PLoc> {
            pinf_header: reader.read_bytes()?,
            message: reader.read_bytes::<PLoc>()?,
        };
        return Ok(msg);
    }
}

impl ReadFromBytes for PNam {
    fn read_from_bytes<R: ReadBytesExt>(mut reader: R) -> io::Result<Self> {
        let name = reader.read_bytes()?;
        let pnam = PNam { name };
        Ok(pnam)
    }
}

impl ReadFromBytes for PLoc {
    fn read_from_bytes<R: ReadBytesExt>(mut reader: R) -> io::Result<Self> {
        let listening_tcp_port = reader.read_u16::<LE>()?;
        let kind = reader.read_bytes()?;
        let name = reader.read_bytes()?;
        let state = reader.read_bytes()?;
        let ploc = PLoc {
            listening_tcp_port,
            kind,
            name,
            state,
        };
        Ok(ploc)
    }
}

impl SizeBytes for PNam {
    fn size_bytes(&self) -> usize {
        self.name.size_bytes()
    }
}

impl SizeBytes for PLoc {
    fn size_bytes(&self) -> usize {
        mem::size_of::<u16>()
            + self.kind.size_bytes()
            + self.name.size_bytes()
            + self.state.size_bytes()
    }
}

#[test]
fn test_ploc_message_read_bytes() {
    let ploc_packet: [u8; 96] = [
        0x43, 0x49, 0x54, 0x50, 0x01, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x50, 0x49, 0x4e, 0x46, 0x50, 0x4c, 0x6f, 0x63, 0x4a, 0xfa, 0x56, 0x69, 0x73, 0x75,
        0x61, 0x6c, 0x69, 0x7a, 0x65, 0x72, 0x00, 0x43, 0x61, 0x70, 0x74, 0x75, 0x72, 0x65, 0x20,
        0x40, 0x20, 0x48, 0x75, 0x67, 0x6f, 0x73, 0x2d, 0x4d, 0x61, 0x63, 0x42, 0x6f, 0x6f, 0x6b,
        0x2d, 0x50, 0x72, 0x6f, 0x2e, 0x6c, 0x6f, 0x63, 0x61, 0x6c, 0x20, 0x28, 0x31, 0x39, 0x32,
        0x2e, 0x31, 0x36, 0x38, 0x2e, 0x31, 0x36, 0x38, 0x2e, 0x38, 0x30, 0x29, 0x00, 0x52, 0x75,
        0x6e, 0x6e, 0x69, 0x6e, 0x67, 0x00,
    ];
    let buffer = ploc_packet.to_vec();

    let citp_header = buffer.as_slice().read_bytes::<Message<PLoc>>();

    assert!(citp_header.is_ok());
    assert_eq!(
        citp_header.unwrap().pinf_header.content_type.to_le_bytes(),
        *b"PLoc"
    );
}
