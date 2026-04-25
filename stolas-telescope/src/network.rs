use bytes::{
    Buf,
    BufMut,
    BytesMut,
};
use color_eyre::eyre::Error;
use futures_util::SinkExt;
use stolas_core::{
    Config,
    Frame,
};
use tokio::{
    net::TcpListener,
    sync::broadcast,
};
use tokio_stream::StreamExt;
use tokio_util::{
    codec::{
        Framed,
        LengthDelimitedCodec,
    },
    sync::CancellationToken,
};

const TAG_CONFIG: u16 = 0x0001;
const TAG_SHUTDOWN: u16 = 0x0002;
const TAG_FRAME: u16 = 0x0003;

pub async fn handle_network(
    listen_address: String,
    config: Config,
    mut signal_receiver: broadcast::Receiver<Frame>,
    shutdown: CancellationToken,
) -> Result<(), Error> {
    let listener = TcpListener::bind(&listen_address).await?;

    'outer: loop {
        tracing::info!(%listen_address, "Waiting for connection");
        let (stream, client_address) = tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("Shutdown signal. Stopping network.");
                break;
            }
            result = listener.accept() => result?,
        };
        tracing::info!(%client_address, "Client connected");

        let codec = LengthDelimitedCodec::new();
        let mut framed = Framed::new(stream, codec);

        // send packet with current state to client
        // todo: actually send state (json)
        let mut send_buffer = BytesMut::new();
        send_buffer.put_u16_ne(TAG_CONFIG);
        serde_json::to_writer((&mut send_buffer).writer(), &config)?;
        framed.send(send_buffer.freeze()).await?;
        framed.flush().await?;

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("Shutdown signal. Stopping network.");
                    break 'outer;
                }
                packet = framed.try_next() => {
                    let Some(mut packet) = packet? else {
                        tracing::info!("Connection closed by remote peer");
                        break;
                    };
                    let command = packet.get_u16_ne();
                    match command {
                        TAG_CONFIG => {
                            // todo: accept configuration
                        }
                        TAG_SHUTDOWN => {
                            break 'outer;
                        }
                        _ => {
                            tracing::warn!("Received unrecognized command: {command:04x}");
                        }
                    }
                }
                frame = signal_receiver.recv() => {
                    let Ok(frame) = frame else {
                        tracing::error!("Signal channel closed");
                        break 'outer;
                    };

                    let mut send_buffer = BytesMut::new();
                    send_buffer.put_u16_ne(TAG_FRAME);

                    frame.write((&mut send_buffer).writer())?;
                    framed.send(send_buffer.freeze()).await?;
                    framed.flush().await?;
                }
            }
        }
    }

    Ok(())
}
