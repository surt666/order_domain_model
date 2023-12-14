use crate::entities::*;
use rust_fsm::*;

/*
events/state        | InProgress | Payed    | Sent                     | Delivered | PayDiff | DeliveryFailed | Failed |
ItemAdded           | InProgress | PayDiff  | Failed                   | Failed     | PayDiff  | Failed       | Failed |
ItemDeleted         | InProgress | PayDiff  | Failed                   | Failed     | PayDiff  | Failed       | Failed |
OrderPayed          | Payed      | Failed   | Failed                   | Failed     | Payed    | Failed       | Failed |
OrderDetailsAdded   | InProgress | Failed   | Failed                   | Failed     | Failed   | Failed       | Failed |
OrderSent           | Failed     | Sent     | Failed                   | Failed     | Failed   | Failed       | Failed |
OrderDelivered      | Failed     | Failed   | Delivered                | Failed     | Failed   | Failed       | Failed |
OrderDeliveryFailed | Failed     | Failed   | DeliveryFailed [ReSend]  | Failed     | Failed   | Failed       | Failed |
CustomerAdded       | InProgress | Failed   | Failed                   | Failed     | Failed   | Failed       | Failed |
*/
state_machine! {
  derive(Debug)
  OrderState(InProgress)

  InProgress(OrderPayed) => Payed [PrepareOrder],
  Payed => {
    OrderSent => Sent,
    ItemAdded => PayDiff,
    ItemDeleted => PayDiff,
    OrderPayed => Failed,
    OrderDetailsAdded => Failed,
    OrderDelivered => Failed,
    OrderDeliveryFailed => Failed,
    CustomerAdded => Failed,
  },
  Sent => {
    OrderDelivered => Delivered,
    OrderDeliveryFailed => DeliveryFailed [ContactCustomer],
  },
  PayDiff(OrderPayed) => Payed [PrepareOrder],
}

fn aggregate_order(mut events: Vec<OrderEvent>, mut order: Order, mut machine: StateMachine<OrderState>) -> Order {
  if events.is_empty() {
    return order;
  }

  match events.first().unwrap() {
    OrderEvent::ItemAdded { id, order_id, time: _ } => {
      order.id = order_id.clone();
      order.items.push(id.clone());
      let _ = machine.consume(&OrderStateInput::ItemAdded);
    }
    OrderEvent::ItemDeleted { id, order_id, time: _ } => {
      order.id = order_id.clone();
      let _ = machine.consume(&OrderStateInput::ItemDeleted);
      match machine.state() {
        OrderStateState::InProgress => {
          if let Some(pos) = order.items.iter().position(|item_id| item_id == id) {
            order.items.remove(pos);
          }
        }
        OrderStateState::PayDiff => {
          order.status = Status::PayDiff;
        }
        _ => {
          order.status = Status::Failed;
        }
      }
    }

    OrderEvent::OrderPayed { order_id, payment_type, amount, time: _ } => {
      order.id = order_id.clone();
      let output = machine.consume(&OrderStateInput::OrderPayed).unwrap();
      if let Some(OrderStateOutput::PrepareOrder) = output {
        order.status = Status::Payed;
        order.action = Action::PrepareOrder;
      }
      order.payment_type = Some(*payment_type);
      order.amount = *amount;
    }
    OrderEvent::OrderDetailsAdded { order_id, delivery_type, delivery_address, customer, time: _ } => {
      order.id = order_id.clone();
      let _ = machine.consume(&OrderStateInput::OrderDetailsAdded);
      order.delivery_type = Some(*delivery_type);
      if delivery_address.is_some() {
        order.address = delivery_address.clone();
      }
      if order.customer.is_none() {
        order.customer = Some(customer.clone());
      }
    }
    OrderEvent::OrderSent { order_id, time: _ } => {
      order.id = order_id.clone();
      let _ = machine.consume(&OrderStateInput::OrderSent);
      if let OrderStateState::Sent = machine.state() {
        order.status = Status::Sent;
        order.action = Action::None;
      } else {
        order.status = Status::Failed;
        order.action = Action::CheckOrder;
      }
    }
    OrderEvent::OrderDelivered { order_id: _, time: _ } => {
      let _ = machine.consume(&OrderStateInput::OrderDelivered);
      if let OrderStateState::Delivered = machine.state() {
        order.status = Status::Delivered;
        order.action = Action::None;
      } else {
        order.status = Status::Failed;
        order.action = Action::CheckOrder;
      }
    }
    OrderEvent::OrderDeliveryFailed { order_id: _, reason: _, time: _ } => {
      let output = machine.consume(&OrderStateInput::OrderDeliveryFailed).unwrap();
      if let Some(OrderStateOutput::ContactCustomer) = output {
        order.action = Action::ContactCustomer;
        if let OrderStateState::DeliveryFailed = machine.state() {
          order.status = Status::DeliveryFailed
        } else {
          order.status = Status::Failed;
          order.action = Action::CheckOrder;
        }
      }
    }
    OrderEvent::CustomerAdded { customer, first_name: _, last_name: _, address, time: _ } => {
      let _ = machine.consume(&OrderStateInput::CustomerAdded);
      if order.address.is_none() {
        order.address = Some(address.clone());
      }
      order.customer = Some(customer.clone());
    }
  }
  events.remove(0);
  aggregate_order(events, order, machine)
}

#[cfg(test)]
mod tests {
  use rust_fsm::StateMachine;

  use crate::{
    entities::{Action, Address, CountryCode, DeliveryType, Order, OrderEvent, PaymentType, Reason, ReasonCode, Status},
    logic::aggregate_order,
  };

  #[test]
  fn aggregate_test() {
    let order = Order {
      id: "1234".to_string(),
      status: Status::Delivered,
      payment_type: Some(PaymentType::VISA),
      amount: 345.34,
      delivery_type: Some(DeliveryType::GLS),
      items: vec!["1234".to_string(), "2345".to_string()],
      address: Some(Address { street: "Karisevej", house_number: 43, zip: 4690, country: CountryCode::DK }),
      customer: Some("765432".to_string()),
      action: Action::None,
    };
    let events = vec![
      OrderEvent::ItemAdded { id: "1234".to_string(), order_id: "1234".to_string(), time: 1 },
      OrderEvent::ItemAdded { id: "2345".to_string(), order_id: "1234".to_string(), time: 2 },
      OrderEvent::ItemAdded { id: "3456".to_string(), order_id: "1234".to_string(), time: 3 },
      OrderEvent::ItemDeleted { id: "3456".to_string(), order_id: "1234".to_string(), time: 4 },
      OrderEvent::CustomerAdded {
        customer: "765432".to_string(),
        first_name: "Steen".to_string(),
        last_name: "Larsen".to_string(),
        address: Address { street: "Taagevej", house_number: 43, zip: 4600, country: CountryCode::DK },
        time: 0,
      },
      OrderEvent::OrderDetailsAdded {
        order_id: "1234".to_string(),
        delivery_type: DeliveryType::GLS,
        delivery_address: Some(Address { street: "Karisevej", house_number: 43, zip: 4690, country: CountryCode::DK }),
        customer: "54321".to_string(),
        time: 5,
      },
      OrderEvent::OrderPayed { order_id: "1234".to_string(), payment_type: PaymentType::VISA, amount: 345.34, time: 6 },
      OrderEvent::OrderSent { order_id: "1234".to_string(), time: 7 },
      OrderEvent::OrderDelivered { order_id: "1234".to_string(), time: 8 },
    ];
    assert_eq!(aggregate_order(events, Order::new("1234".to_string()), StateMachine::new()), order);
  }

  #[test]
  fn aggregate_test_no_delivery_address() {
    let order = Order {
      id: "1234".to_string(),
      status: Status::Delivered,
      payment_type: Some(PaymentType::VISA),
      amount: 345.34,
      delivery_type: Some(DeliveryType::GLS),
      items: vec!["1234".to_string(), "2345".to_string()],
      address: Some(Address { street: "Taagevej", house_number: 43, zip: 4600, country: CountryCode::DK }),
      customer: Some("765432".to_string()),
      action: Action::None,
    };
    let events = vec![
      OrderEvent::ItemAdded { id: "1234".to_string(), order_id: "1234".to_string(), time: 1 },
      OrderEvent::ItemAdded { id: "2345".to_string(), order_id: "1234".to_string(), time: 2 },
      OrderEvent::ItemAdded { id: "3456".to_string(), order_id: "1234".to_string(), time: 3 },
      OrderEvent::ItemDeleted { id: "3456".to_string(), order_id: "1234".to_string(), time: 4 },
      OrderEvent::CustomerAdded {
        customer: "765432".to_string(),
        first_name: "Steen".to_string(),
        last_name: "Larsen".to_string(),
        address: Address { street: "Taagevej", house_number: 43, zip: 4600, country: CountryCode::DK },
        time: 0,
      },
      OrderEvent::OrderDetailsAdded {
        order_id: "1234".to_string(),
        delivery_type: DeliveryType::GLS,
        delivery_address: None,
        customer: "54321".to_string(),
        time: 5,
      },
      OrderEvent::OrderPayed { order_id: "1234".to_string(), payment_type: PaymentType::VISA, amount: 345.34, time: 6 },
      OrderEvent::OrderSent { order_id: "1234".to_string(), time: 7 },
      OrderEvent::OrderDelivered { order_id: "1234".to_string(), time: 8 },
    ];
    assert_eq!(aggregate_order(events, Order::new("1234".to_string()), StateMachine::new()), order);
  }

  #[test]
  fn aggregate_test_fail_delivery() {
    let order = Order {
      id: "1234".to_string(),
      status: Status::DeliveryFailed,
      payment_type: Some(PaymentType::VISA),
      amount: 345.34,
      delivery_type: Some(DeliveryType::GLS),
      items: vec!["1234".to_string(), "2345".to_string()],
      address: Some(Address { street: "Karisevej", house_number: 43, zip: 4690, country: CountryCode::DK }),
      customer: Some("54321".to_string()),
      action: Action::ContactCustomer,
    };
    let events = vec![
      OrderEvent::ItemAdded { id: "1234".to_string(), order_id: "1234".to_string(), time: 1 },
      OrderEvent::ItemAdded { id: "2345".to_string(), order_id: "1234".to_string(), time: 2 },
      OrderEvent::ItemAdded { id: "3456".to_string(), order_id: "1234".to_string(), time: 3 },
      OrderEvent::ItemDeleted { id: "3456".to_string(), order_id: "1234".to_string(), time: 4 },
      OrderEvent::OrderDetailsAdded {
        order_id: "1234".to_string(),
        delivery_type: DeliveryType::GLS,
        delivery_address: Some(Address { street: "Karisevej", house_number: 43, zip: 4690, country: CountryCode::DK }),
        customer: "54321".to_string(),
        time: 5,
      },
      OrderEvent::OrderPayed { order_id: "1234".to_string(), payment_type: PaymentType::VISA, amount: 345.34, time: 6 },
      OrderEvent::OrderSent { order_id: "1234".to_string(), time: 7 },
      OrderEvent::OrderDeliveryFailed {
        order_id: "1234".to_string(),
        reason: Reason { reason_code: ReasonCode::PackageLost, reason_message: "Package went into the sea".to_string() },
        time: 8,
      },
    ];
    assert_eq!(aggregate_order(events, Order::new("1234".to_string()), StateMachine::new()), order);
  }
}
