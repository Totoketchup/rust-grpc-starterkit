mod protos;
mod shared;

#[macro_use]
extern crate log;

use std::{
    io::{self, Read},
    sync::Arc,
    thread,
    time::Duration,
};

use futures::{stream, sync::oneshot, Future, Sink, Stream};
use grpcio::{
    ChannelBuilder, ClientStreamingSink, DuplexSink, Environment, Error, RequestStream, ResourceQuota,
    RpcContext, ServerBuilder, ServerStreamingSink, UnarySink, WriteFlags,
};
use rand;

use crate::protos::{
    mathematician::{
        GeneratorReply, GeneratorRequest, SumReply, SumRequest,
        SumStreamRequest, CalculationRequest, Type
    },
    mathematician_grpc::{create_mathematician, Mathematician},
};

use crate::shared::log_utils;

struct GeneratorIter;
impl Iterator for GeneratorIter {
    type Item = GeneratorReply;
    fn next(&mut self) -> Option<Self::Item> {
        std::thread::sleep(Duration::from_millis(1000));
        let mut number = GeneratorReply::new();
        number.set_number(rand::random::<i32>());
        Some(number)
    }
}

#[derive(Clone)]
struct MathematicianService;

impl Mathematician for MathematicianService {

    fn compute_sum(&mut self, ctx: RpcContext<'_>, req: SumRequest, sink: UnarySink<SumReply>) {
        let mut resp = SumReply::default();
        let a = req.get_a();
        let b = req.get_b();
        resp.set_sum(a + b);
        let f = sink
            .success(resp)
            .map_err(move |e| error!("failed to reply {:?}: {:?}", req, e));
        ctx.spawn(f)
    }

    fn number_generator(
        &mut self,
        ctx: RpcContext<'_>,
        req: GeneratorRequest,
        sink: ServerStreamingSink<GeneratorReply>,
    ) {
        let number_generator = GeneratorIter {};
        let iter = number_generator
            .take(req.get_seed() as usize)
            .map(|e| (e, WriteFlags::default()));

        let f = sink
            .send_all(stream::iter_ok::<_, Error>(iter))
            .map(|_| {})
            .map_err(|e| println!("failed to handle the stream: {:?}", e));
        ctx.spawn(f)
    }

    fn sum_stream(
        &mut self,
        ctx: RpcContext<'_>,
        stream: RequestStream<SumStreamRequest>,
        sink: ClientStreamingSink<SumReply>,
    ) {
        let f = stream
            .fold(0, move |mut acc, value_message| {
                info!("Message received: {:?}", value_message);
                acc += value_message.get_value();
                Ok(acc) as Result<_, grpcio::Error>
            })
            .and_then(|sum| {
                let mut resp = SumReply::default();
                resp.set_sum(sum);
                sink.success(resp)
            })
            .map_err(|e| error!("SumStreamError: Failed to reply: {}", e));
        ctx.spawn(f)
    }

    fn calculation(
        &mut self,
        ctx: RpcContext<'_>,
        stream: RequestStream<CalculationRequest>,
        sink: DuplexSink<SumStreamRequest>,
    ) {
        let mut current_calculation = 0;
        let response_stream = stream
            .map(move |value_message| {
                match value_message.get_field_type() {
                    Type::ADD => {
                        current_calculation += value_message.get_value();
                    },
                    Type::MULTIPLY => {
                        current_calculation *= value_message.get_value();
                    },
                    Type::SUBTRACT => {
                        current_calculation -= value_message.get_value();
                    },
                }
                let mut resp = SumStreamRequest::new();
                resp.set_value(current_calculation);
                (resp, WriteFlags::default())
            });
        let f = sink
            .send_all(response_stream)
            .map(|_| {})
            .map_err(|e| error!("failed to reply: {:?}", e));
        ctx.spawn(f)
    }
}

fn main() {
    log_utils::init();

    let env = Arc::new(Environment::new(1));
    let service = create_mathematician(MathematicianService);

    let quota = ResourceQuota::new(Some("MathematicianServerQuota")).resize_memory(1024 * 1024);
    let ch_builder = ChannelBuilder::new(env.clone()).resource_quota(quota);

    let mut server = ServerBuilder::new(env)
        .register_service(service)
        .bind("127.0.0.1", 50_051)
        .channel_args(ch_builder.build_args())
        .build()
        .unwrap();
    server.start();
    for &(ref host, port) in server.bind_addrs() {
        info!("listening on {}:{}", host, port);
    }
    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        info!("Press ENTER to exit...");
        let _ = io::stdin().read(&mut [0]).unwrap();
        tx.send(())
    });
    let _ = rx.wait();
    let _ = server.shutdown().wait();
}
