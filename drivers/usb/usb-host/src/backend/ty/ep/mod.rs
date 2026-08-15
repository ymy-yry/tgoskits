#[cfg(any(kmod, umod))]
use alloc::vec::Vec;
use alloc::{boxed::Box, sync::Arc};
use core::{
    any::Any,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use ax_sync::SpinLock;
#[cfg(any(kmod, umod))]
use usb_if::endpoint::{IsoPacketResult, TransferStatus};
use usb_if::{
    descriptor::EndpointType,
    endpoint::{EndpointInfo, RequestId, TransferCompletion, TransferRequest},
    err::TransferError,
};

#[cfg(any(kmod, umod))]
use super::transfer::Transfer;

mod ctrl;

pub(crate) trait EndpointOp: Send + Any + 'static {
    fn submit_request(&mut self, request: TransferRequest) -> Result<RequestId, TransferError>;

    fn reclaim_request(
        &mut self,
        id: RequestId,
    ) -> Option<Result<TransferCompletion, TransferError>>;

    fn register_waker(&self, id: RequestId, cx: &mut Context<'_>);

    fn cancel_request(&mut self, _id: RequestId) -> Result<(), TransferError> {
        Err(TransferError::NotSupported)
    }

    fn reset(&mut self) -> EndpointResetFuture {
        Box::pin(async { Err(TransferError::NotSupported) })
    }
}

pub type EndpointResetFuture =
    Pin<Box<dyn Future<Output = Result<(), TransferError>> + Send + 'static>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EndpointLifecycle {
    Active,
    Revoked,
    Disconnected,
}

struct EndpointInner {
    raw: Box<dyn EndpointOp>,
    lifecycle: EndpointLifecycle,
}

#[derive(Clone)]
pub struct EndpointHandle {
    info: EndpointInfo,
    inner: Arc<SpinLock<EndpointInner>>,
}

impl EndpointHandle {
    #[cfg(any(kmod, umod))]
    pub(crate) fn new(info: EndpointInfo, raw: impl EndpointOp) -> Self {
        Self {
            info,
            inner: Arc::new(SpinLock::new(EndpointInner {
                raw: Box::new(raw),
                lifecycle: EndpointLifecycle::Active,
            })),
        }
    }

    pub fn info(&self) -> EndpointInfo {
        self.info
    }

    pub fn submit(&self, request: TransferRequest) -> Result<RequestId, TransferError> {
        self.validate_request(&request)?;
        let mut inner = self.inner.lock();
        match inner.lifecycle {
            EndpointLifecycle::Active => inner.raw.submit_request(request),
            EndpointLifecycle::Revoked => Err(TransferError::EndpointRevoked),
            EndpointLifecycle::Disconnected => Err(TransferError::Disconnected),
        }
    }

    pub fn reclaim(&self, id: RequestId) -> Result<Option<TransferCompletion>, TransferError> {
        match self.inner.lock().raw.reclaim_request(id) {
            Some(result) => result.map(Some),
            None => Ok(None),
        }
    }

    pub fn poll_request(
        &self,
        id: RequestId,
        cx: &mut Context<'_>,
    ) -> Poll<Result<TransferCompletion, TransferError>> {
        let mut inner = self.inner.lock();
        match inner.raw.reclaim_request(id) {
            Some(res) => Poll::Ready(res),
            None => {
                inner.raw.register_waker(id, cx);
                match inner.raw.reclaim_request(id) {
                    Some(res) => Poll::Ready(res),
                    None => Poll::Pending,
                }
            }
        }
    }

    pub fn cancel(&self, id: RequestId) -> Result<(), TransferError> {
        self.inner.lock().raw.cancel_request(id)
    }

    /// Resets host-controller state for this endpoint after a successful
    /// `CLEAR_FEATURE(ENDPOINT_HALT)` request.
    pub fn reset(&self) -> EndpointResetFuture {
        self.inner.lock().raw.reset()
    }

    pub async fn wait(
        &self,
        request: TransferRequest,
    ) -> Result<TransferCompletion, TransferError> {
        let id = self.submit(request)?;
        EndpointRequestFuture {
            id,
            endpoint: self,
            completed: false,
        }
        .await
    }

    #[allow(unused)]
    pub(crate) fn with_raw_mut<T: EndpointOp, R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut inner = self.inner.lock();
        let d = inner.raw.as_mut() as &mut dyn Any;
        f(d.downcast_mut::<T>()
            .expect("EndpointHandle downcast_mut failed"))
    }

    pub(crate) fn revoke(&self) {
        self.inner.lock().lifecycle = EndpointLifecycle::Revoked;
    }

    pub(crate) fn disconnect(&self) {
        self.inner.lock().lifecycle = EndpointLifecycle::Disconnected;
    }

    pub(crate) fn reactivate(&self) {
        self.inner.lock().lifecycle = EndpointLifecycle::Active;
    }

    fn validate_request(&self, request: &TransferRequest) -> Result<(), TransferError> {
        let request_type = match request {
            TransferRequest::Control { .. } => EndpointType::Control,
            TransferRequest::Bulk { .. } => EndpointType::Bulk,
            TransferRequest::Interrupt { .. } => EndpointType::Interrupt,
            TransferRequest::Isochronous { .. } => EndpointType::Isochronous,
        };
        if request_type == self.info.transfer_type {
            Ok(())
        } else {
            Err(TransferError::InvalidEndpoint)
        }
    }
}

struct EndpointRequestFuture<'a> {
    id: RequestId,
    endpoint: &'a EndpointHandle,
    completed: bool,
}

impl Future for EndpointRequestFuture<'_> {
    type Output = Result<TransferCompletion, TransferError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let result = this.endpoint.poll_request(this.id, cx);
        if result.is_ready() {
            this.completed = true;
        }
        result
    }
}

impl Drop for EndpointRequestFuture<'_> {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self.endpoint.cancel(self.id);
        }
    }
}

#[cfg(any(kmod, umod))]
pub(crate) fn transfer_to_completion(id: RequestId, transfer: Transfer) -> TransferCompletion {
    let iso_packets = match &transfer.kind {
        usb_if::endpoint::TransferKind::Isochronous { packet_lengths } => packet_lengths
            .iter()
            .copied()
            .zip(transfer.iso_packet_actual_lengths.iter().copied())
            .zip(transfer.iso_packet_completion_codes.iter().copied())
            .map(
                |((requested_length, actual_length), completion_code)| IsoPacketResult {
                    requested_length,
                    actual_length,
                    status: TransferStatus::Completed,
                    completion_code,
                },
            )
            .collect(),
        _ => Vec::new(),
    };

    TransferCompletion {
        request_id: id,
        status: TransferStatus::Completed,
        actual_length: transfer.transfer_len,
        iso_packets,
    }
}

#[cfg(all(test, any(kmod, umod)))]
mod tests {
    use alloc::boxed::Box;
    use core::{
        pin::Pin,
        task::{Context, Poll, Waker},
    };

    use usb_if::{
        endpoint::{EndpointAddress, EndpointInfo, RequestId, TransferRequest},
        transfer::Direction,
    };

    use super::{EndpointHandle, EndpointOp, EndpointType, TransferCompletion, TransferError};

    struct PendingEndpoint {
        cancelled: bool,
    }

    impl EndpointOp for PendingEndpoint {
        fn submit_request(
            &mut self,
            _request: TransferRequest,
        ) -> Result<RequestId, TransferError> {
            Ok(RequestId::new(7))
        }

        fn reclaim_request(
            &mut self,
            _id: RequestId,
        ) -> Option<Result<TransferCompletion, TransferError>> {
            None
        }

        fn register_waker(&self, _id: RequestId, _cx: &mut Context<'_>) {}

        fn cancel_request(&mut self, id: RequestId) -> Result<(), TransferError> {
            assert_eq!(id, RequestId::new(7));
            self.cancelled = true;
            Ok(())
        }
    }

    #[test]
    fn dropping_pending_wait_cancels_the_inflight_request() {
        let info = EndpointInfo {
            address: EndpointAddress::new(1),
            transfer_type: EndpointType::Bulk,
            direction: Direction::Out,
            max_packet_size: 64,
            packets_per_microframe: 1,
            interval: 0,
        };
        let endpoint = EndpointHandle::new(info, PendingEndpoint { cancelled: false });
        let mut future = Box::pin(endpoint.wait(TransferRequest::bulk_out(&[1, 2, 3])));
        let mut context = Context::from_waker(Waker::noop());

        assert!(matches!(
            Pin::new(&mut future).poll(&mut context),
            Poll::Pending
        ));
        drop(future);

        assert!(endpoint.with_raw_mut::<PendingEndpoint, _>(|raw| raw.cancelled));
    }
}
