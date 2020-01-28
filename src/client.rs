mod protos;
mod shared;

#[macro_use]
extern crate log;

use std::sync::Arc;

use crate::protos::{
	helloworld::{
		GeneratorReply, GeneratorRequest, HelloRequest, SumReply, SumRequest, SumStreamRequest,
	},
	helloworld_grpc::GreeterClient,
};
use crate::shared::log_utils;
use futures::{Sink, Stream};
use grpcio::{
	ChannelBuilder, ClientCStreamReceiver, ClientSStreamReceiver, EnvBuilder, StreamingCallSink,
	WriteFlags,
};

fn main() {
	log_utils::init();

	let env = Arc::new(EnvBuilder::new().build());
	let ch = ChannelBuilder::new(env).connect("localhost:50051");
	let client = GreeterClient::new(ch);

	// Hello world test
	let mut req = HelloRequest::default();
	req.set_name("Anthony !".to_owned());
	let reply = client.say_hello(&req).expect("rpc");
	info!("Greeter received: {}", reply.get_message());

	// Sum test
	let mut req = SumRequest::default();
	req.set_a(2);
	req.set_b(40);
	let reply = client.compute_sum(&req).expect("rpc");
	info!("Sum received: {}", reply.get_sum());

	// Generator test
	let mut req_generator = GeneratorRequest::default();
	req_generator.set_seed(5);
	let reply: ClientSStreamReceiver<GeneratorReply> =
		client.number_generator(&req_generator).expect("rpc");
	for (index, message) in reply.wait().enumerate() {
		if let Ok(number) = message {
			info!("{} - Received number: {:?}", index, number.get_number());
		} else {
			info!("{:?}", message);
		}
	}

	// Sum stream test
	// let values_test: Vec<i32> = vec![1, 4, 5, 6];

	// let (stream_request, stream_receiver): (
	// 	StreamingCallSink<SumStreamRequest>,
	// 	ClientCStreamReceiver<SumReply>,
	// ) = client
	// 	.sum_stream()
	// 	.unwrap_or_else(|err| panic!("Error while creating sum stream client func: {}", err));

	// let iter_send = values_test.iter().map(|i| {
	// 	let mut req_sumstream = SumStreamRequest::default();
	// 	req_sumstream.set_value(*i);
	// 	req_sumstream
	// }).collect::<Vec<_>>().iter();

	// stream_request.send_all((iter_send, WriteFlags::default()));
}
