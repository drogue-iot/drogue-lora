#![no_std]
#![feature(generic_associated_types)]

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use drogue_lora::*;
use embedded_hal::{
    blocking::{
        delay::DelayMs,
        spi::{Transfer, Write},
    },
    digital::v2::{InputPin, OutputPin},
};
use heapless::{consts, Vec};

use lorawan_device::{
    radio, region, Device as LorawanDevice, Error as LorawanError, Event as LorawanEvent,
    Region as LorawanRegion, Response as LorawanResponse, Timings as RadioTimings,
};
use lorawan_encoding::default_crypto::DefaultFactory as Crypto;

mod sx127x_lora;
mod sx127x_radio;

use sx127x_radio::{RadioPhyEvent, Sx127xRadio as Radio};

enum DriverState<SPI, CS, RESET, E>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E> + 'static,
    CS: OutputPin + 'static,
    RESET: OutputPin + 'static,
    E: 'static,
{
    Initialized(Radio<SPI, CS, RESET, E>),
    Configured(LorawanDevice<Radio<SPI, CS, RESET, E>, Crypto>),
}

pub struct Sx127xDriver<SPI, CS, RESET, E>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E> + 'static,
    CS: OutputPin + 'static,
    RESET: OutputPin + 'static,
    E: 'static,
{
    state: Option<DriverState<SPI, CS, RESET, E>>,
    //   response: Option<&'static Signal<ControllerResponse>>,
    get_random: fn() -> u32,
}

impl<SPI, CS, RESET, E> Sx127xDriver<SPI, CS, RESET, E>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E> + 'static,
    CS: OutputPin + 'static,
    RESET: OutputPin + 'static,
{
    pub fn new(
        spi: SPI,
        cs: CS,
        reset: RESET,
        delay: &mut dyn DelayMs<u8>,
        get_random: fn() -> u32,
    ) -> Result<Self, LoraError> {
        let radio = Radio::new(spi, cs, reset, delay)?;
        log::info!("Radio created!");
        Ok(Self {
            state: Some(DriverState::Initialized(radio)),
            get_random,
        })
    }

    async fn process_event(&mut self, event: LorawanEvent<'static, Radio<SPI, CS, RESET, E>>) {
        match self.state.take().unwrap() {
            DriverState::Configured(lorawan) => {
                match &event {
                    LorawanEvent::NewSessionRequest => {
                        log::trace!("New Session Request");
                    }
                    LorawanEvent::RadioEvent(e) => match e {
                        radio::Event::TxRequest(_, _) => (),
                        radio::Event::RxRequest(_) => (),
                        radio::Event::CancelRx => (),
                        radio::Event::PhyEvent(phy) => {
                            // log::info!("Phy event");
                        }
                    },
                    LorawanEvent::TimeoutFired => (),
                    LorawanEvent::SendDataRequest(_e) => {
                        log::trace!("SendData");
                    }
                }
                // log_stack("Handling event");
                let (mut new_state, response) = lorawan.handle_event(event);
                // log::info!("Event handled");
                self.process_response(&mut new_state, response);
                self.state.replace(DriverState::Configured(new_state));
            }
            s => {
                log::info!("Not yet configured, event processing skipped");
                self.state.replace(s);
            }
        }
    }

    fn process_response(
        &self,
        lorawan: &mut LorawanDevice<Radio<SPI, CS, RESET, E>, Crypto>,
        response: Result<LorawanResponse, LorawanError<Radio<SPI, CS, RESET, E>>>,
    ) {
        match response {
            Ok(response) => match response {
                LorawanResponse::TimeoutRequest(ms) => {
                    log::trace!("TimeoutRequest: {:?}", ms);
                    /*self.scheduler.as_ref().unwrap().schedule(
                        Milliseconds(ms),
                        LorawanEvent::TimeoutFired,
                        self.me.as_ref().unwrap().clone(),
                    );*/
                }
                LorawanResponse::JoinSuccess => {
                    // log::trace!("Join Success: {:?}", lorawan.get_session_keys().unwrap());
                    // self.response.as_ref().unwrap().signal(Ok(None));
                }
                LorawanResponse::ReadyToSend => {
                    log::trace!("RxWindow expired but no ACK expected. Ready to Send");
                }
                LorawanResponse::DownlinkReceived(fcnt_down) => {
                    if let Some(downlink) = lorawan.take_data_downlink() {
                        let fhdr = downlink.fhdr();
                        let fopts = fhdr.fopts();
                        use lorawan_encoding::parser::{DataHeader, FRMPayload};

                        if let Ok(FRMPayload::Data(data)) = downlink.frm_payload() {
                            log::trace!(
                                "Downlink received \t\t(FCntDown={}\tFRM: {:?})",
                                fcnt_down,
                                data,
                            );
                            //let mut v = Vec::new();
                            //v.extend_from_slice(data);
                            //     self.response.as_ref().unwrap().signal(Ok(Some(v)));
                        } else {
                            //    self.response.as_ref().unwrap().signal(Ok(None));
                            log::trace!("Downlink received \t\t(FcntDown={})", fcnt_down);
                        }

                        let mut mac_commands_len = 0;
                        for mac_command in fopts {
                            if mac_commands_len == 0 {
                                log::trace!("\tFOpts: ");
                            }
                            // log::trace!("{:?},", mac_command);
                            mac_commands_len += 1;
                        }
                    }
                }
                LorawanResponse::NoAck => {
                    log::trace!("RxWindow expired, expected ACK to confirmed uplink not received");
                    //self.response.as_ref().unwrap().signal(Ok(None));
                }
                LorawanResponse::NoJoinAccept => {
                    log::info!("No Join Accept Received. Retrying.");
                    /*self.me
                    .as_ref()
                    .unwrap()
                    .notify(LorawanEvent::NewSessionRequest);*/
                }
                LorawanResponse::SessionExpired => {
                    log::info!("SessionExpired. Created new Session");
                    /*self.me
                    .as_ref()
                    .unwrap()
                    .notify(LorawanEvent::NewSessionRequest);*/
                }
                LorawanResponse::NoUpdate => {
                    // log::info!("No update");
                }
                LorawanResponse::UplinkSending(fcnt_up) => {
                    log::trace!("Uplink with FCnt {}", fcnt_up);
                }
                LorawanResponse::JoinRequestSending => {
                    log::trace!("Join Request Sending");
                }
            },
            Err(err) => match err {
                LorawanError::Radio(_) => log::error!("Radio error"),
                LorawanError::Session(e) => log::error!("Session error"), //{:?}", e),
                LorawanError::NoSession(_) => log::error!("NoSession error"),
            },
        }
    }

    /// Process the interrupt for an event
    pub async fn handle_interrupt(&mut self) {
        self.process_event(LorawanEvent::RadioEvent(radio::Event::PhyEvent(
            RadioPhyEvent::Irq,
        )))
        .await
    }
}

pub struct Sx127xConfigure<'a, SPI, CS, RESET, E>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E> + 'static,
    CS: OutputPin + 'static,
    RESET: OutputPin + 'static,
    E: 'static,
{
    state: &'a mut Option<DriverState<SPI, CS, RESET, E>>,
    config: &'a LoraConfig,
    get_random: fn() -> u32,
}

impl<'a, SPI, CS, RESET, E> Future for Sx127xConfigure<'a, SPI, CS, RESET, E>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E> + 'static,
    CS: OutputPin + 'static,
    RESET: OutputPin + 'static,
{
    type Output = Result<(), LoraError>;
    fn poll(mut self: core::pin::Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state.take().unwrap() {
            DriverState::Initialized(radio) => {
                //log::info!("Configuring radio");
                let dev_eui = self
                    .config
                    .device_eui
                    .as_ref()
                    .expect("device EUI must be set");
                let app_eui = self.config.app_eui.as_ref().expect("app EUI must be set");
                let app_key = self.config.app_key.as_ref().expect("app KEY must be set");
                //log::info!("Creating device");
                let lorawan: LorawanDevice<Radio<SPI, CS, RESET, E>, Crypto> = LorawanDevice::new(
                    region::EU868::default().into(),
                    radio,
                    dev_eui.reverse().into(),
                    app_eui.reverse().into(),
                    app_key.clone().into(),
                    self.get_random,
                );
                self.state.replace(DriverState::Configured(lorawan));
                // self.response.as_ref().unwrap().signal(Ok(None));
                Poll::Ready(Ok(()))
            }
            other => {
                //log::info!("Driver not yet initialized, ignoring configuration");
                self.state.replace(other);
                Poll::Ready(Err(LoraError::OtherError))
            }
        }
    }
}

impl<SPI, CS, RESET, E> LoraDriver for Sx127xDriver<SPI, CS, RESET, E>
where
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E> + 'static,
    CS: OutputPin + 'static,
    RESET: OutputPin + 'static,
{
    type ConfigureFuture<'a> = Sx127xConfigure<'a, SPI, CS, RESET, E>;
    fn configure<'a>(&'a mut self, config: &'a LoraConfig) -> Self::ConfigureFuture<'a> {
        Sx127xConfigure {
            config,
            state: &mut self.state,
            get_random: self.get_random,
        }
    }

    /*
    fn reset(self, message: Reset) -> Response<Self, Result<(), LoraError>> {
        Response::immediate(self, Err(LoraError::OtherError))
    }

    fn join(mut self, message: Join) -> Response<Self, Result<(), LoraError>> {
        Response::defer(async move {
            self.process_event(LorawanEvent::NewSessionRequest).await;
            (self, Ok(()))
        })
    }

    fn send<'a>(self, message: Send<'a>) -> Response<Self, Result<(), LoraError>> {
        unsafe {
            Response::defer_unchecked(async move {
                let state = self.state.as_ref().unwrap();
                match state.take() {
                    State::Configured(lorawan) => {
                        let ready_to_send = lorawan.ready_to_send_data();
                        state.replace(if ready_to_send {
                            let (mut new_state, response) = lorawan.send(
                                message.2,
                                message.1,
                                match message.0 {
                                    QoS::Confirmed => true,
                                    QoS::Unconfirmed => false,
                                },
                            );
                            self.process_response(&mut new_state, response);
                            State::Configured(new_state)
                        } else {
                            State::Configured(lorawan)
                        });
                        (self, Ok(()))
                    }
                    other => {
                        //log::info!("Driver not yet initialized, ignoring configuration");
                        state.replace(other);
                        (self, Err(LoraError::OtherError))
                    }
                }
            })
        }
    }

    fn send_recv<'a>(self, message: SendRecv<'a>) -> Response<Self, Result<usize, LoraError>> {
        Response::immediate(self, Err(LoraError::NotImplemented))
    }*/
}

/*
impl<S, SPI, CS, RESET, BUSY, DELAY, E>
    NotifyHandler<LorawanEvent<'static, Radio<SPI, CS, RESET, E>>>
    for Sx127xController<S, SPI, CS, RESET, BUSY, DELAY, E>
where
    S: Scheduler,
    SPI: Transfer<u8, Error = E> + Write<u8, Error = E> + 'static,
    CS: OutputPin + 'static,
    RESET: OutputPin + 'static,
    BUSY: InputPin,
    DELAY: DelayMs<u8>,
{
    fn on_notify(
        mut self,
        message: LorawanEvent<'static, Radio<SPI, CS, RESET, E>>,
    ) -> Completion<Self> {
        Completion::defer(async move {
            self.process_event(message).await;
            self
        })
    }
}

pub struct Sx127xInterrupt<H, RADIO, READY>
where
    RADIO: radio::PhyRxTx + RadioTimings,
    H: NotifyHandler<LorawanEvent<'static, RADIO>> + 'static,
    READY: InterruptPin + 'static,
{
    controller: Option<Address<H>>,
    ready: READY,
    _phantom: core::marker::PhantomData<RADIO>,
}

impl<H, RADIO, READY> Sx127xInterrupt<H, RADIO, READY>
where
    RADIO: radio::PhyRxTx + RadioTimings,
    H: NotifyHandler<LorawanEvent<'static, RADIO>>,
    READY: InterruptPin + 'static,
{
    pub fn new(ready: READY) -> Self {
        Self {
            ready,
            controller: None,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<H, RADIO, READY> Actor for Sx127xInterrupt<H, RADIO, READY>
where
    RADIO: radio::PhyRxTx + RadioTimings,
    H: NotifyHandler<LorawanEvent<'static, RADIO>>,
    READY: InterruptPin + 'static,
{
    type Configuration = Address<H>;

    fn on_mount(&mut self, me: Address<Self>, config: Self::Configuration) {
        self.controller.replace(config);
    }
}

impl<H, RADIO, READY> Interrupt for Sx127xInterrupt<H, RADIO, READY>
where
    RADIO: radio::PhyRxTx<PhyEvent = RadioPhyEvent> + RadioTimings + 'static,
    H: NotifyHandler<LorawanEvent<'static, RADIO>>,
    READY: InterruptPin + 'static,
{
    fn on_interrupt(&mut self) {
        if self.ready.check_interrupt() {
            self.ready.clear_interrupt();
            self.controller
                .as_ref()
                .unwrap()
                .notify(LorawanEvent::RadioEvent(radio::Event::PhyEvent(
                    RadioPhyEvent::Irq,
                )));
        }
    }
}

struct DriverResponse {
    signal: &'static Signal<ControllerResponse>,
}

impl DriverResponse {
    pub fn new(signal: &'static Signal<ControllerResponse>) -> Self {
        Self { signal }
    }
}

impl core::future::Future for DriverResponse {
    type Output = ControllerResponse;

    fn poll(mut self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.signal.poll_wait(cx)
    }
}
*/
