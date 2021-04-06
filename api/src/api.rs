use crate::types::*;
use core::future::Future;

/// API for accessing LoRa modules
pub trait LoraDriver {
    type ConfigureFuture<'a>: Future<Output = Result<(), LoraError>>
    where
        Self: 'a;
    /// Configure the LoRa module with the provided config.
    fn configure<'a>(&'a mut self, config: &'a LoraConfig) -> Self::ConfigureFuture<'a>;

    /*
    type ResetFuture: Future<Output = Result<(), LoraError>>;
    /// Reset the LoRa module.
    fn reset(&mut self, mode: ResetMode) -> Self::ResetFuture;

    type JoinFuture: Future<Output = Result<(), LoraError>>;
    /// Join a LoRaWAN network with the given connect mode.
    fn join(&mut self, mode: ConnectMode) -> Self::JoinFuture;

    type SendFuture<'a>: Future<Output = Result<(), LoraError>>
    where
        Self: 'a;
    /// Send data on a specific port with a given quality of service.
    fn send<'a>(&mut self, qos: QoS, port: Port, data: &'a [u8]) -> Self::SendFuture<'a>;

    type SendRecvFuture<'a>: Future<Output = Result<usize, LoraError>>
    where
        Self: 'a;
    /// Send data on a specific port with a given quality of service. If the LoRa module receives
    /// any data as part of the confirmation, write it into the provided buffer and return the size of the data read.
    fn send_recv<'a>(
        &mut self,
        qos: QoS,
        port: Port,
        data: &'a [u8],
        rx: &'a mut [u8],
    ) -> Self::SendRecvFuture<'a>;*/
}

#[derive(Debug)]
pub enum LoraError {
    SendError,
    RecvError,
    RecvTimeout,
    RecvBufferTooSmall,
    NotInitialized,
    NotImplemented,
    OtherError,
}
