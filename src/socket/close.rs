use std::borrow::Cow;

use axum::extract::ws::{CloseFrame, Message};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
#[allow(dead_code)]
pub enum CloseCode {
	NormalClosure = 1000,
	GoingAway,
	ProtocolError,
	UnsupportedData,
	InvalidFramePayloadData = 1007,
	PolicyViolation,
	MessageTooBig,
	MandatoryExt,
	InternalError,
	ServiceRestart,
	TryAgainLater,
	InvalidResponse,
	Unauthorized = 3000,
}

impl CloseCode {
	pub fn into_close_message(self, reason: &'static str) -> Message {
		Message::Close(Some(CloseFrame {
			code: self as _,
			reason: Cow::Borrowed(reason),
		}))
	}
}
