mod protos;
mod shared;

#[macro_use]
extern crate log;

use std::{sync::Arc, thread};

use crate::protos::{
	mathematician::{
		CalculationRequest, GeneratorReply, GeneratorRequest,
		SumReply, SumRequest, SumStreamRequest, Type,
	},
	mathematician_grpc::MathematicianClient,
};
use crate::shared::log_utils;
use futures::{future, Future, Sink, Stream};
use grpcio::{
	ChannelBuilder, ClientCStreamReceiver, ClientSStreamReceiver, 
	ClientDuplexReceiver, EnvBuilder, StreamingCallSink,
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
		info!("Sending value {:?}", sending_value);
		stream_request = stream_request
			.send((sending_value, WriteFlags::default()))
			.wait()
			.unwrap();
	}
	stream_request
		.close()
		.expect("Error while closing the Sum Stream Request");

	info!("Receiving Sum...");
	let response: i32 = sum_receiver
		.wait()
		.map(|result: protos::mathematician::SumReply| result.get_sum())
		.expect("Couldn't receive the sum response");

	info!("Received Sum Stream Response: {}", response);

	// Bidirectional Calculation 
	let values_test = vec![1, 2, 3, 4, 5, 6, 7, 8];
	let type_test = vec![
		Type::ADD,
		Type::ADD,
		Type::SUBTRACT,
		Type::MULTIPLY,
		Type::ADD,
		Type::SUBTRACT,
		Type::SUBTRACT,
		Type::SUBTRACT,
	];
	let requests_test = values_test
		.iter()
		.enumerate()
		.map(|(i, &val)| {
			let mut calc_req = CalculationRequest::new();
			calc_req.set_value(val);
			calc_req.set_field_type(type_test[i]);
			calc_req
		})
		.collect::<Vec<CalculationRequest>>();
	
	let (mut stream_request, mut stream_receiver): (
		StreamingCallSink<CalculationRequest>,
		ClientDuplexReceiver<SumStreamRequest>,
	) = client
		.calculation()
		.unwrap_or_else(|err| panic!("Error while creating calculation stream client func: {}", err));

	let sending_thread = thread::spawn(move || {
		for request in requests_test {
			info!("Sending: {:?}", request);
			stream_request = stream_request.send((request, WriteFlags::default())).wait().unwrap();
		}
		future::poll_fn(|| stream_request.close()).wait().unwrap();
	});

	loop {
        match stream_receiver.into_future().wait() {
            Ok((Some(message), r)) => {
                let value = message.get_value();
                info!("Received value {}", value);
                stream_receiver = r;
            }
            Ok((None, _)) => break,
            Err((e, _)) => panic!("Calculation RPC failed: {:?}", e),
        }
    }

	sending_thread.join().unwrap();
}
