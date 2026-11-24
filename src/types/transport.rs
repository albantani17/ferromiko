use std::io::{Read, Write};

/// A non-blocking shell stream: reads return `WouldBlock` when no data is waiting.
pub trait Transport: Read + Write + Send {}

impl<T: Read + Write + Send> Transport for T {}
