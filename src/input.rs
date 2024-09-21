use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error;

use futures::channel::mpsc;
use futures::channel::mpsc::UnboundedReceiver;
use futures::Stream as FuturesStream;
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
            let stdin = io::stdin();
            let input = stdin.events();

            for res_key in input {
                match res_key {
                    Ok(key) => {
                        if let Err(e) = tx.unbounded_send(key) {
                            return Err(error::Error::from(e));
                        }
                    }
                    Err(e) => {
                        println!("{:?}", e);
                        closed_handle.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }

            Ok(())
        });

        AsyncKeyInput {
            rx,
            closed,
            handle: Some(handle),
        }
    }
}

impl FuturesStream for AsyncKeyInput {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.closed.load(Ordering::SeqCst) {
            match self.handle.take().expect("AsyncKeyInput was missing its thread handle.").join() {
                Ok(Ok(())) => Poll::Ready(None),
                Ok(Err(e)) => panic!("Error: {e}"),
                Err(e) => panic!("Error: {e:?}"),
            }
        } else {
            match Pin::new(&mut self.rx).poll_next(cx) {
                Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}
