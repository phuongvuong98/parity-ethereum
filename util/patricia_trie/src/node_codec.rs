use bytes::*;
use nibbleslice::NibbleSlice;
//use nibblevec::NibbleVec;
use rlp::{self, Prototype, Rlp, RlpStream, DecoderError, Decodable, Encodable};
use hashdb::Hasher;
use node::Node;
use std::marker::PhantomData;
//pub trait OldNodeCodec<'a>: Sized {
//	type Encoding;
//	type StreamEncoding;
//
//	fn encoded(&self) -> Bytes;
//	fn decoded(data: &'a [u8]) -> Result<Self, DecoderError>;
//	fn try_decode_hash<O>(data: &[u8]) -> Option<O> where O: Decodable;
//
//	fn new_encoded(data: &'a[u8]) -> Self::Encoding;
//	fn encoded_stream() -> Self::StreamEncoding;
//	fn encoded_list(size: usize) -> Self::StreamEncoding;
//}


pub trait NodeCodec<H: Hasher>: Sized {
	fn encode(&Node) -> Bytes;
	fn decode(data: &[u8]) -> Result<Node, DecoderError>; // TODO: make the error generic here, perhaps an associated type on the trait
	fn try_decode_hash(data: &[u8]) -> Option<H::Out>;

	// TODO: We don't want these here, but where do they go? Helper trait?
	fn new_encoded<'a>(data: &'a [u8]) -> Rlp<'a>;
	fn encoded_stream() -> RlpStream;
	fn encoded_list(size: usize) -> RlpStream;

}
pub struct RlpNodeCodec<H: Hasher> {mark: PhantomData<H>}

impl<H: Hasher> NodeCodec<H> for RlpNodeCodec<H> where H::Out: Encodable + Decodable {
	fn encode(node: &Node) -> Bytes {
		match *node {
			Node::Leaf(ref slice, ref value) => {
				let mut stream = RlpStream::new_list(2);
				stream.append(&&*slice.encoded(true));
				stream.append(value);
				stream.out()
			},
			Node::Extension(ref slice, ref raw_rlp) => {
				let mut stream = RlpStream::new_list(2);
				stream.append(&&*slice.encoded(false));
				stream.append_raw(raw_rlp, 1);
				stream.out()
			},
			Node::Branch(ref nodes, ref value) => {
				let mut stream = RlpStream::new_list(17);
				for i in 0..16 {
					stream.append_raw(nodes[i], 1);
				}
				match *value {
					Some(ref n) => { stream.append(n); },
					None => { stream.append_empty_data(); },
				}
				stream.out()
			},
			Node::Empty => {
				let mut stream = RlpStream::new();
				stream.append_empty_data();
				stream.out()
			}
		}
	}
	fn decode(data: &[u8]) -> Result<Node, DecoderError> {
		let r = Rlp::new(data);
		match r.prototype()? {
			// either leaf or extension - decode first item with NibbleSlice::???
			// and use is_leaf return to figure out which.
			// if leaf, second item is a value (is_data())
			// if extension, second item is a node (either SHA3 to be looked up and
			// fed back into this function or inline RLP which can be fed back into this function).
			Prototype::List(2) => match NibbleSlice::from_encoded(r.at(0)?.data()?) {
				(slice, true) => Ok(Node::Leaf(slice, r.at(1)?.data()?)),
				(slice, false) => Ok(Node::Extension(slice, r.at(1)?.as_raw())),
			},
			// branch - first 16 are nodes, 17th is a value (or empty).
			Prototype::List(17) => {
				let mut nodes = [&[] as &[u8]; 16];
				for i in 0..16 {
					nodes[i] = r.at(i)?.as_raw();
				}
				Ok(Node::Branch(nodes, if r.at(16)?.is_empty() { None } else { Some(r.at(16)?.data()?) }))
			},
			// an empty branch index.
			Prototype::Data(0) => Ok(Node::Empty),
			// something went wrong.
			_ => Err(DecoderError::Custom("Rlp is not valid."))
		}
	}
	fn try_decode_hash(data: &[u8]) -> Option<H::Out> {
		let r = Rlp::new(data);
		if r.is_data() && r.size() == 32 {
			Some(r.as_val().expect("Hash is the correct size of 32 bytes; qed"))
		} else {
			None
		}
	}

	fn new_encoded<'a>(data: &'a [u8]) -> Rlp<'a> {
		Rlp::new(data)
	}

	fn encoded_stream() -> RlpStream {
		RlpStream::new()
	}

	fn encoded_list(size: usize) -> RlpStream{
		RlpStream::new_list(size)
	}

}