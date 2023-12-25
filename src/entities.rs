use std::cmp::Ordering;

pub type OrderId = String;
pub type OrderItemId = String;
pub type CustomerId = String;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd)]
pub enum PaymentType {
    VISA,
    MASTERCARD,
    AMERICANEXPRESS,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd)]
pub enum DeliveryType {
    GLS,
    UPS,
    BRING,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Status {
    InProgress,
    Payed,
    PayDiff,
    Sent,
    Delivered,
    DeliveryFailed,
    Failed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd)]
pub enum CountryCode {
    DK,
    US,
    DE,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd)]
pub enum ReasonCode {
    PackageLost,
    WrongAddress,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct Reason {
    pub reason_code: ReasonCode,
    pub reason_message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    None,
    ContactCustomer,
    PrepareOrder,
    CheckOrder,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct Address {
    pub street: &'static str,
    pub house_number: i16,
    pub zip: i16,
    pub country: CountryCode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub id: OrderId,
    pub status: Status,
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
            status: Status::InProgress,
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
