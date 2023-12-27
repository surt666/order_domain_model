use std::cmp::Ordering;

// use strum::IntoEnumIterator;
use strum_macros::{EnumDiscriminants, EnumIter};

pub type OrderId = String;
pub type OrderItemId = String;
pub type CustomerId = String;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Default, Hash)]
pub enum PaymentType {
    #[default]
    VISA,
    MASTERCARD,
    AMERICANEXPRESS,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Default, Hash)]
pub enum DeliveryType {
    #[default]
    GLS,
    UPS,
    BRING,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter, Hash))]
pub enum OrderEvent {
    ItemAdded {
        id: OrderItemId,
        order_id: OrderId,
        time: u32,
    },
    ItemDeleted {
        id: OrderItemId,
        order_id: OrderId,
        time: u32,
    },
    OrderPayed {
        order_id: OrderId,
        payment_type: PaymentType,
        amount: u32,
        time: u32,
    },
    OrderDetailsAdded {
        order_id: OrderId,
        delivery_type: DeliveryType,
        delivery_address: Option<Address>,
        customer: CustomerId,
        time: u32,
    },
    OrderSent {
        order_id: OrderId,
        time: u32,
    },
    OrderDelivered {
        order_id: OrderId,
        time: u32,
    },
    OrderDeliveryFailed {
        order_id: OrderId,
        reason: Reason,
        time: u32,
    },
    CustomerAdded {
        customer: CustomerId,
        first_name: String,
        last_name: String,
        address: Address,
        time: u32,
    },
}

#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for OrderEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        println!("ORD");
        use OrderEvent::*;
        let get_time = |event: &OrderEvent| -> u32 {
            match event {
                ItemAdded { time, .. }
                | ItemDeleted { time, .. }
                | OrderPayed { time, .. }
                | OrderDetailsAdded { time, .. }
                | OrderSent { time, .. }
                | OrderDelivered { time, .. }
                | OrderDeliveryFailed { time, .. }
                | CustomerAdded { time, .. } => *time,
            }
        };
        println!("S {} O {}", get_time(self), get_time(other));
        get_time(self).cmp(&get_time(other))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, EnumIter, Hash)]
pub enum State {
    #[default]
    Empty,
    InProgress,
    Payed,
    PayDiff,
    Sent,
    Delivered,
    DeliveryFailed,
    Failed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Default, Hash)]
pub enum CountryCode {
    #[default]
    DK,
    US,
    DE,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Default, Hash)]
pub enum ReasonCode {
    #[default]
    PackageLost,
    WrongAddress,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default, Hash)]
pub struct Reason {
    pub reason_code: ReasonCode,
    pub reason_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, Default)]
pub enum Action {
    #[default]
    None,
    AddItem,
    DeleteItem,
    Pay,
    RefundDiff,
    ContactCustomer,
    PrepareOrder,
    CheckOrder,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Default, Hash)]
pub struct Address {
    pub street: &'static str,
    pub house_number: i16,
    pub zip: i16,
    pub country: CountryCode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub id: OrderId,
    pub status: State,
    pub payment_type: Option<PaymentType>,
    pub amount: u32,
    pub delivery_type: Option<DeliveryType>,
    pub items: Vec<OrderItemId>,
    pub address: Option<Address>,
    pub customer: Option<CustomerId>,
    pub action: Action,
}

impl Order {
    pub fn new(id: String) -> Order {
        Order {
            id,
            status: State::Empty,
            items: vec![],
            address: None,
            customer: None,
            delivery_type: None,
            amount: 0,
            payment_type: None,
            action: Action::None,
        }
    }
}
