// following this tutorial:
// https://ryhl.io/blog/actors-with-tokio/

use tokio::sync::{oneshot, mpsc};

struct MyActor {
    receiver: mpsc::Receiver<ActorMessage>
}

enum ActorMessage {
    GetId {
        respond_to: oneshot::Sender<u32>
    }
}

impl MyActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        Self { receiver }
    }
    async fn run(&mut self) {
        //when all senders have been dropped, .recv() below will receive a None, which means we'll exit the while loop and drop the receiver as well
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg);
            println!("im here")
        }
    }
    fn handle_message(&mut self, msg: ActorMessage) {
        //doing a match here - but could also be doing a match in run_my_actor
        match msg {
            // here we list all the diff messages possible
            ActorMessage::GetId { respond_to } => {
                // we drop the errors here. Error happens if the original sender no longer cares about the result of the operation
                let _ = respond_to.send(123);
            },
            _ => unreachable!()
        }
    }
}


// can derive clone here because the channel is multi producer, single consumer
#[derive(Clone)]
pub struct MyActorHandle {
    // handle has the sender, the actor has the receiver
    sender: mpsc::Sender<ActorMessage>,
}

/// Note this is the only public interface to the actor system that we're exposing
impl MyActorHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(8); //todo how bounded do I want it?
        //first call new()
        let mut actor = MyActor::new(receiver);
        //then call run()
        tokio::spawn(async move{ actor.run().await });
        Self { sender }
    }
    pub async fn get_id(&self) -> u32 {
        let (oneoff_sender, oneoff_receiver) = oneshot::channel();

        //send method on a bounded channel does not return immediately
        //send method on a one-off channel DOES return immediately

        //if you don't want to hear back and
        // the channel can get full - you need to make this an async function
        // otherwise use try_send and handle sending failures by killing the actor

        let _ = self.sender.send(ActorMessage::GetId {respond_to: oneoff_sender}).await;
        oneoff_receiver.await.expect("actor task has been killed and failed to respond")
    }
}


#[tokio::main]
pub async fn main() {
    let handle = MyActorHandle::new();
    let id = handle.get_id().await;
    println!("{}", id);
    let id = handle.get_id().await;
    println!("{}", id);
    let id = handle.get_id().await;
    println!("{}", id);
}