use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::pin::Pin;
use std::task::Poll;

use crate::error;

use tokio_stream::Stream;
use futures::{Sink, Stream as FuturesStream, Async};
use futures::sync::mpsc;
use futures::sync::mpsc::UnboundedReceiver;
use termion::event::Event;
use termion::input::TermRead;

pub struct AsyncKeyInput {
    rx: UnboundedReceiver<Event>,
    closed: Arc<AtomicBool>,
    handle: Option<JoinHandle<error::Result<()>>>,
}

impl AsyncKeyInput {
    pub fn new() -> AsyncKeyInput {
        let (tx, rx) = mpsc::unbounded();
        let closed = Arc::new(AtomicBool::new(false));
        let closed_handle = closed.clone();

        let handle: JoinHandle<error::Result<()>> = thread::spawn(move || {
            let mut tx = &tx;
            let stdin = io::stdin();
            let input = stdin.events();

            for res_key in input {
                match res_key {
                    Ok(key) => {
                        if let Err(e) = tx.unbounded_send(key) {
                            return Err(e.into())
                        }
                    }
                    Err(e) => {
                        println!("{:?}", e);
                        if let Err(e) = tx.close() {
                            return Err(e.into())
                        }
                        closed_handle.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }

            Ok(())
        });

        AsyncKeyInput {
            rx: rx,
            closed,
            handle: Some(handle),
        }
    }
}

impl Stream for AsyncKeyInput {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        if self.closed.load(Ordering::SeqCst) {
            match self.handle.take().expect("AsyncKeyInput was missing its thread handle.").join() {
                Ok(Ok(())) => Poll::Ready(None),
                Ok(Err(e)) => panic!("Error: {e}"),
                Err(e) => panic!("Error: {e:?}"),
            }
        } else {
            match self.rx.poll().map_err(|()| unreachable!()) {
                Ok(item) => {
                    match item {
                        Async::Ready(item) => Poll::Ready(item),
                        Async::NotReady => Poll::Pending,
                    }
                }
                Err(e) => panic!("Error: {e:?}"),
            }
        }
    }
}
