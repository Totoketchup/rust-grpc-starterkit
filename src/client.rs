mod protos;
mod shared;

#[macro_use]
extern crate log;

use std::{sync::Arc, thread};

use crate::protos::{
	mathematician::{
		GeneratorReply, GeneratorRequest, SumReply, SumRequest, SumStreamRequest,
	},
	mathematician_grpc::MathematicianClient,
};
use crate::shared::log_utils;
use futures::{future, Future, Sink, Stream};
use grpcio::{
	ChannelBuilder, ClientCStreamReceiver, ClientSStreamReceiver, EnvBuilder, StreamingCallSink,
	WriteFlags,
};

fn main() {
	log_utils::init();

	let env = Arc::new(EnvBuilder::new().build());
	let ch = ChannelBuilder::new(env).connect("localhost:50051");
	let client = MathematicianClient::new(ch);

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
	let values_test: Vec<i32> = vec![1, 4, 5, 6];

	let (mut stream_request, sum_receiver): (
		StreamingCallSink<SumStreamRequest>,
		ClientCStreamReceiver<SumReply>,
	) = client
		.sum_stream()
		.unwrap_or_else(|err| panic!("Error while creating sum stream client func: {}", err));

	for value in values_test {
		let mut sending_value = SumStreamRequest::new();
		sending_value.set_value(value);

		stream_request = stream_request
			.send((sending_value, WriteFlags::default()))
			.wait()
			.unwrap();
	}
	stream_request
		.close()
		.expect("Error while closing the Sum Stream Request");

	let response: i32 = sum_receiver
		.wait()
		.map(|result: protos::mathematician::SumReply| result.get_sum())
		.expect("Couldn't receive the sum response");

	info!("Received Sum Stream Response: {}", response);
}
