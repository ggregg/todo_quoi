//! Fabrique des « tampons » de modification (qui, quand).

use crate::ports::Clock;
use std::sync::Arc;
use tq_domain::{ChangeStamp, DeviceId};

/// Associe l'horloge et l'identité de cet appareil pour tamponner chaque modification.
// [rust] `Arc<dyn Clock>` : pointeur partagé vers « n'importe quelle horloge » (ADR-0003).
//        `#[derive(Clone)]` sur une struct contenant un `Arc` copie le pointeur, pas l'horloge.
#[derive(Clone)]
pub struct Stamper {
    clock: Arc<dyn Clock>,
    device: DeviceId,
}

impl Stamper {
    /// Crée le tampon de cet appareil.
    pub fn new(clock: Arc<dyn Clock>, device: DeviceId) -> Self {
        Self { clock, device }
    }

    /// Tampon « maintenant, par cet appareil ».
    pub fn stamp(&self) -> ChangeStamp {
        ChangeStamp {
            at: self.clock.now(),
            by: self.device.clone(),
        }
    }
}
