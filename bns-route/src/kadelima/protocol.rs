//use crate::kadelima::candidates::{Address, Candidate};
//use serde::{Deserialize, Serialize};
//use std::net::UdpSocket;
//use std::str;
//use std::sync::mpsc::Sender;
//use std::sync::Arc;
//use std::thread;

//#[derive(Serialize, Deserialize, Debug, Clone)]
//pub enum RequestPayload {
//Store(Address, String),
//FindCandidate(Address),
//FindValue(Address),
//}

//#[derive(Serialize, Deserialize, Debug, Clone)]
//pub enum ResponsePayload {
//Candidates(Vec<Candidate>),
//Value(String),
//}

//#[derive(Serialize, Deserialize, Debug)]
//pub enum Message {
//Request(Request),
//Response(Response),
//Kill,
//}

//#[derive(Serialize, Deserialize, Debug, Clone)]
//pub struct Request {
//pub id: Address,
//pub sender: Candidate,
//pub payload: RequestPayload,
//}

//#[derive(Serialize, Deserialize, Debug, Clone)]
//pub struct Response {
//pub request: Request,
//pub receiver: Candidate,
//pub payload: ResponsePayload,
//}

//#[derive(Clone)]
//pub struct Protocol {
//swarm: Arc<Swarm>,
//}
