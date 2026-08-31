// use httparse::{Header, Request};
// use sansio::Protocol;

// struct HTTP {
//     routs: Vec<u8>,
//     headers: [Header]
// }

// impl<'a> Protocol<&'a [u8], (), ()> for HTTP {
//     type Rout = Request<'static, 'a>;

//     type Wout = ();

//     type Eout = ();

//     type Error = ();

//     type Time = ();

//     fn handle_read(&mut self, msg: &'a [u8]) -> Result<(), Self::Error> {
//         todo!()
//     }

//     fn poll_read(&mut self) -> Option<Self::Rout> {
//         todo!()
//     }

//     fn handle_write(&mut self, msg: ()) -> Result<(), Self::Error> {
//         todo!()
//     }

//     fn poll_write(&mut self) -> Option<Self::Wout> {
//         todo!()
//     }
// }
