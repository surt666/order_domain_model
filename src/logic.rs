use crate::entities::{Action, Order, OrderEvent, OrderEventDiscriminants, State};
use fsm::{StateMachine, TStateMachine};
// use strum_macros::EnumIter;

#[allow(clippy::too_many_lines)]
pub fn aggregate_order(
    mut events: Vec<OrderEvent>, mut order: Order, machine: &mut StateMachine<State, OrderEventDiscriminants, Action>,
) -> Order {
    if events.is_empty() {
        return order;
    } else if let Some(event) = events.first() {
        match event {
            OrderEvent::ItemAdded { id, order_id, time } => {
                println!("ItemAdded");
                order.id = order_id.clone();
                order.items.push(id.clone());
                machine.update_state(OrderEventDiscriminants::ItemAdded);
                let state = machine.current_state();
                println!("State {:#?}", state.state);
            }
            OrderEvent::ItemDeleted { id, order_id, time } => {
                println!("ItemDeleted");
                order.id = order_id.clone();
                machine.update_state(OrderEventDiscriminants::ItemDeleted);
                let state = machine.current_state();
                println!("State {:#?}", state.state);
                match state.state {
                    State::InProgress => {
                        if let Some(pos) = order.items.iter().position(|item_id| item_id == id) {
                            order.items.remove(pos);
                        }
                    }
                    State::PayDiff => {
                        order.status = State::PayDiff;
                    }
                    _ => {
                        order.status = State::Failed;
                    }
                }
            }
            OrderEvent::OrderPayed { order_id, payment_type, amount, time } => {
                println!("OrderPayed");
                order.id = order_id.clone();
                machine.update_state(OrderEventDiscriminants::OrderPayed);
                let state = machine.current_state();
                if state.actions.contains(&Action::PrepareOrder) {
                    order.status = State::Payed;
                    order.action = Action::PrepareOrder;
                }
                order.payment_type = Some(*payment_type);
                order.amount = *amount;
            }
            OrderEvent::OrderDetailsAdded { order_id, delivery_type, delivery_address, customer, time } => {
                println!("OrderDetailsAdded");
                order.id = order_id.clone();
                machine.update_state(OrderEventDiscriminants::OrderDetailsAdded);
                order.delivery_type = Some(*delivery_type);
                if delivery_address.is_some() {
                    order.address = delivery_address.clone();
                }
                if order.customer.is_none() {
                    order.customer = Some(customer.clone());
                }
            }
            OrderEvent::OrderSent { order_id, time } => {
                println!("OrderSent");
                order.id = order_id.clone();
                machine.update_state(OrderEventDiscriminants::OrderSent);
                let state = machine.current_state();
                println!("State {:#?}", state.state);
                if state.state == State::Sent {
                    order.status = State::Sent;
                    order.action = Action::None;
                } else {
                    order.status = State::Failed;
                    order.action = Action::CheckOrder;
                }
            }
            OrderEvent::OrderDelivered { order_id, time } => {
                println!("OrderDelivered");
                machine.update_state(OrderEventDiscriminants::OrderDelivered);
                let state = machine.current_state();
                println!("State {:#?}", state.state);
                if state.state == State::Delivered {
                    order.status = State::Delivered;
                    order.action = Action::None;
                } else {
                    order.status = State::Failed;
                    order.action = Action::CheckOrder;
                }
            }
            OrderEvent::OrderDeliveryFailed { order_id, reason, time } => {
                println!("OrderDeliveryFailed");
                machine.update_state(OrderEventDiscriminants::OrderDeliveryFailed);
                let state = machine.current_state();
                println!("State {:#?}", state.state);
                if state.actions.contains(&Action::ContactCustomer) {
                    order.action = Action::ContactCustomer;
                }
                if State::DeliveryFailed == state.state {
                    order.status = State::DeliveryFailed;
                } else {
                    order.status = State::Failed;
                    order.action = Action::CheckOrder;
                }
            }
            OrderEvent::CustomerAdded { customer, first_name, last_name, address, time } => {
                println!("CustomerAdded");
                machine.update_state(OrderEventDiscriminants::CustomerAdded);
                let state = machine.current_state();
                println!("State {:#?}", state.state);
                if order.address.is_none() {
                    order.address = Some(address.clone());
                }
                order.customer = Some(customer.clone());
            }
        }
    }
    events.remove(0);
    aggregate_order(events, order, machine)
}

pub fn add_event(event: OrderEvent, store_fn: fn(OrderEvent) -> Vec<OrderEvent>) -> Vec<OrderEvent> {
    let mut events = store_fn(event);
    events.sort_by(std::cmp::Ord::cmp);
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entities::{Action, Address, CountryCode, DeliveryType, Order, OrderEvent, PaymentType, Reason, ReasonCode, State},
        logic::{add_event, aggregate_order},
    };
    use fsm::{StateMachine, StateResult};
    use std::{collections::HashMap, sync::LazyLock};
    use strum::IntoEnumIterator;

    /*
    events/state        | Empty      | InProgress | Payed                    | Sent                               | Delivered  | PayDiff  | DeliveryFailed | Failed |
    ItemAdded           | InProgress | InProgress | PayDiff                  | Failed                             | Failed     | PayDiff  | Failed         | Failed |
    ItemDeleted         | Failed     | InProgress | Payed [RefundDiff]       | Failed                             | Failed     | PayDiff  | Failed         | Failed |
    OrderPayed          | Failed     | Payed      | Failed                   | Failed                             | Failed     | Payed    | Failed         | Failed |
    OrderDetailsAdded   | InProgress | InProgress | Failed                   | Failed                             | Failed     | Failed   | Failed         | Failed |
    OrderSent           | Failed     | Failed     | Sent                     | Failed                             | Failed     | Failed   | Failed         | Failed |
    OrderDelivered      | Failed     | Failed     | Failed                   | Delivered                          | Failed     | Failed   | Failed         | Failed |
    OrderDeliveryFailed | Failed     | Failed     | Failed                   | DeliveryFailed [ContactCustomer]   | Failed     | Failed   | Failed         | Failed |
    CustomerAdded       | InProgress | InProgress | Failed                   | Failed                             | Failed     | Failed   | Failed         | Failed |
    */

    static TRANSITIONS: LazyLock<HashMap<(OrderEventDiscriminants, State), StateResult<State, Action>>> = LazyLock::new(|| {
        let mut map: HashMap<(OrderEventDiscriminants, State), StateResult<State, Action>> = HashMap::new();

        /* ItemAdded */
        map.insert(
            (OrderEventDiscriminants::ItemAdded, State::Empty),
            StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] },
        );
        map.insert(
            (OrderEventDiscriminants::ItemAdded, State::InProgress),
            StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] },
        );
        map.insert((OrderEventDiscriminants::ItemAdded, State::Payed), StateResult { state: State::PayDiff, actions: vec![Action::Pay] });
        map.insert((OrderEventDiscriminants::ItemAdded, State::PayDiff), StateResult { state: State::PayDiff, actions: vec![Action::Pay] });
        map.insert((OrderEventDiscriminants::ItemAdded, State::Sent), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::ItemAdded, State::Delivered), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::ItemAdded, State::DeliveryFailed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::ItemAdded, State::Failed), StateResult { state: State::Failed, actions: vec![] });
        /* ItemDeleted */
        map.insert(
            (OrderEventDiscriminants::ItemDeleted, State::Empty),
            StateResult { state: State::Failed, actions: vec![Action::AddItem] },
        );
        map.insert(
            (OrderEventDiscriminants::ItemDeleted, State::InProgress),
            StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] },
        );
        map.insert(
            (OrderEventDiscriminants::ItemDeleted, State::Payed),
            StateResult { state: State::Payed, actions: vec![Action::RefundDiff] },
        );
        map.insert(
            (OrderEventDiscriminants::ItemDeleted, State::PayDiff),
            StateResult { state: State::PayDiff, actions: vec![Action::Pay] },
        );
        map.insert((OrderEventDiscriminants::ItemDeleted, State::Sent), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::ItemDeleted, State::Delivered), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::ItemDeleted, State::DeliveryFailed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::ItemDeleted, State::Failed), StateResult { state: State::Failed, actions: vec![] });
        /* OrderPayed */
        map.insert((OrderEventDiscriminants::OrderPayed, State::Empty), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderPayed, State::InProgress), StateResult { state: State::Payed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderPayed, State::Payed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderPayed, State::PayDiff), StateResult { state: State::Payed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderPayed, State::Sent), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderPayed, State::Delivered), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderPayed, State::DeliveryFailed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderPayed, State::Failed), StateResult { state: State::Failed, actions: vec![] });
        /* OrderDetailsAdded */
        map.insert(
            (OrderEventDiscriminants::OrderDetailsAdded, State::Empty),
            StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] },
        );
        map.insert(
            (OrderEventDiscriminants::OrderDetailsAdded, State::InProgress),
            StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] },
        );
        map.insert((OrderEventDiscriminants::OrderDetailsAdded, State::Payed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDetailsAdded, State::PayDiff), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDetailsAdded, State::Sent), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDetailsAdded, State::Delivered), StateResult { state: State::Failed, actions: vec![] });
        map.insert(
            (OrderEventDiscriminants::OrderDetailsAdded, State::DeliveryFailed),
            StateResult { state: State::Failed, actions: vec![] },
        );
        map.insert((OrderEventDiscriminants::OrderDetailsAdded, State::Failed), StateResult { state: State::Failed, actions: vec![] });
        /* OrderSent */
        map.insert((OrderEventDiscriminants::OrderSent, State::Empty), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderSent, State::InProgress), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderSent, State::Payed), StateResult { state: State::Sent, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderSent, State::PayDiff), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderSent, State::Sent), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderSent, State::Delivered), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderSent, State::DeliveryFailed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderSent, State::Failed), StateResult { state: State::Failed, actions: vec![] });
        /* OrderDelivered */
        map.insert((OrderEventDiscriminants::OrderDelivered, State::Empty), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDelivered, State::InProgress), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDelivered, State::Payed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDelivered, State::PayDiff), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDelivered, State::Sent), StateResult { state: State::Delivered, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDelivered, State::Delivered), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDelivered, State::DeliveryFailed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDelivered, State::Failed), StateResult { state: State::Failed, actions: vec![] });
        /* OrderDeliveryFailed */
        map.insert((OrderEventDiscriminants::OrderDeliveryFailed, State::Empty), StateResult { state: State::Failed, actions: vec![] });
        map.insert(
            (OrderEventDiscriminants::OrderDeliveryFailed, State::InProgress),
            StateResult { state: State::Failed, actions: vec![] },
        );
        map.insert((OrderEventDiscriminants::OrderDeliveryFailed, State::Payed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::OrderDeliveryFailed, State::PayDiff), StateResult { state: State::Failed, actions: vec![] });
        map.insert(
            (OrderEventDiscriminants::OrderDeliveryFailed, State::Sent),
            StateResult { state: State::DeliveryFailed, actions: vec![Action::ContactCustomer] },
        );
        map.insert((OrderEventDiscriminants::OrderDeliveryFailed, State::Delivered), StateResult { state: State::Failed, actions: vec![] });
        map.insert(
            (OrderEventDiscriminants::OrderDeliveryFailed, State::DeliveryFailed),
            StateResult { state: State::Failed, actions: vec![] },
        );
        map.insert((OrderEventDiscriminants::OrderDeliveryFailed, State::Failed), StateResult { state: State::Failed, actions: vec![] });
        /* CustomerAdded */
        map.insert(
            (OrderEventDiscriminants::CustomerAdded, State::Empty),
            StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] },
        );
        map.insert(
            (OrderEventDiscriminants::CustomerAdded, State::InProgress),
            StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] },
        );
        map.insert((OrderEventDiscriminants::CustomerAdded, State::Payed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::CustomerAdded, State::PayDiff), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::CustomerAdded, State::Sent), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::CustomerAdded, State::Delivered), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::CustomerAdded, State::DeliveryFailed), StateResult { state: State::Failed, actions: vec![] });
        map.insert((OrderEventDiscriminants::CustomerAdded, State::Failed), StateResult { state: State::Failed, actions: vec![] });
        map
    });

    // static TRANSITIONS: LazyLock<Vec<Vec<StateResult<State, Action>>>> = LazyLock::new(|| {
    //     vec![
    //         vec![
    //             /* ItemAdded */
    //             StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] }, //Empty
    //             StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] }, //InProgress
    //             StateResult { state: State::PayDiff, actions: vec![Action::Pay] },                            //Payed
    //             StateResult { state: State::PayDiff, actions: vec![Action::Pay] },                            //PayDiff
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Sent
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Delivered
    //             StateResult { state: State::Failed, actions: vec![] },                                        //DeliveryFailed
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Failed
    //         ],
    //         vec![
    //             /* ItemDeleted */
    //             StateResult { state: State::Failed, actions: vec![Action::AddItem] }, //Empty
    //             StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] }, //InProgress
    //             StateResult { state: State::Payed, actions: vec![Action::RefundDiff] }, //Payed
    //             StateResult { state: State::PayDiff, actions: vec![Action::Pay] },    //PayDiff
    //             StateResult { state: State::Failed, actions: vec![] },                //Sent
    //             StateResult { state: State::Failed, actions: vec![] },                //Delivered
    //             StateResult { state: State::Failed, actions: vec![] },                //DeliveryFailed
    //             StateResult { state: State::Failed, actions: vec![] },                //Failed
    //         ],
    //         vec![
    //             /* OrderPayed */
    //             StateResult { state: State::Failed, actions: vec![] }, //Empty
    //             StateResult { state: State::Payed, actions: vec![] },  //InProgress
    //             StateResult { state: State::Failed, actions: vec![] }, //Payed
    //             StateResult { state: State::Payed, actions: vec![] },  //PayDiff
    //             StateResult { state: State::Failed, actions: vec![] }, //Sent
    //             StateResult { state: State::Failed, actions: vec![] }, //Delivered
    //             StateResult { state: State::Failed, actions: vec![] }, //DeliveryFailed
    //             StateResult { state: State::Failed, actions: vec![] }, //Failed
    //         ],
    //         vec![
    //             /* OrderDetailsAdded */
    //             StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] }, //Empty
    //             StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] }, //InProgress
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Payed
    //             StateResult { state: State::Failed, actions: vec![] },                                        //PayDiff
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Sent
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Delivered
    //             StateResult { state: State::Failed, actions: vec![] },                                        //DeliveryFailed
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Failed
    //         ],
    //         vec![
    //             /* OrderSent */
    //             StateResult { state: State::Failed, actions: vec![] }, //Empty
    //             StateResult { state: State::Failed, actions: vec![] }, //InProgress
    //             StateResult { state: State::Sent, actions: vec![] },   //Payed
    //             StateResult { state: State::Failed, actions: vec![] }, //PayDiff
    //             StateResult { state: State::Failed, actions: vec![] }, //Sent
    //             StateResult { state: State::Failed, actions: vec![] }, //Delivered
    //             StateResult { state: State::Failed, actions: vec![] }, //DeliveryFailed
    //             StateResult { state: State::Failed, actions: vec![] }, //Failed
    //         ],
    //         vec![
    //             /* OrderDelivered */
    //             StateResult { state: State::Failed, actions: vec![] },    //Empty
    //             StateResult { state: State::Failed, actions: vec![] },    //InProgress
    //             StateResult { state: State::Failed, actions: vec![] },    //Payed
    //             StateResult { state: State::Failed, actions: vec![] },    //PayDiff
    //             StateResult { state: State::Delivered, actions: vec![] }, //Sent
    //             StateResult { state: State::Failed, actions: vec![] },    //Delivered
    //             StateResult { state: State::Failed, actions: vec![] },    //DeliveryFailed
    //             StateResult { state: State::Failed, actions: vec![] },    //Failed
    //         ],
    //         vec![
    //             /* OrderDeliveryFailed */
    //             StateResult { state: State::Failed, actions: vec![] }, //Empty
    //             StateResult { state: State::Failed, actions: vec![] }, //InProgress
    //             StateResult { state: State::Failed, actions: vec![] }, //Payed
    //             StateResult { state: State::Failed, actions: vec![] }, //PayDiff
    //             StateResult { state: State::DeliveryFailed, actions: vec![Action::ContactCustomer] }, //Sent
    //             StateResult { state: State::Failed, actions: vec![] }, //Delivered
    //             StateResult { state: State::Failed, actions: vec![] }, //DeliveryFailed
    //             StateResult { state: State::Failed, actions: vec![] }, //Failed
    //         ],
    //         vec![
    //             /* CustomerAdded */
    //             StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] }, //Empty
    //             StateResult { state: State::InProgress, actions: vec![Action::AddItem, Action::DeleteItem] }, //InProgress
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Payed
    //             StateResult { state: State::Failed, actions: vec![] },                                        //PayDiff
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Sent
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Delivered
    //             StateResult { state: State::Failed, actions: vec![] },                                        //DeliveryFailed
    //             StateResult { state: State::Failed, actions: vec![] },                                        //Failed
    //         ],
    //     ]
    // });

    fn store_event_dummy(event: OrderEvent) -> Vec<OrderEvent> {
        let mut events = vec![
            OrderEvent::ItemAdded { id: "1234".to_string(), order_id: "1234".to_string(), time: 1 },
            OrderEvent::ItemAdded { id: "2345".to_string(), order_id: "1234".to_string(), time: 2 },
            OrderEvent::ItemAdded { id: "3456".to_string(), order_id: "1234".to_string(), time: 3 },
            OrderEvent::ItemDeleted { id: "3456".to_string(), order_id: "1234".to_string(), time: 4 },
            OrderEvent::CustomerAdded {
                customer: "765432".to_string(),
                first_name: "Steen".to_string(),
                last_name: "Larsen".to_string(),
                address: Address { street: "Taagevej", house_number: 43, zip: 4600, country: CountryCode::Dk },
                time: 0,
            },
            OrderEvent::OrderDetailsAdded {
                order_id: "1234".to_string(),
                delivery_type: DeliveryType::Gls,
                delivery_address: Some(Address { street: "Karisevej", house_number: 43, zip: 4690, country: CountryCode::Dk }),
                customer: "54321".to_string(),
                time: 5,
            },
            OrderEvent::OrderPayed { order_id: "1234".to_string(), payment_type: PaymentType::Visa, amount: 345, time: 6 },
            OrderEvent::OrderSent { order_id: "1234".to_string(), time: 7 },
        ];
        events.push(event);
        events
    }

    #[test]
    fn aggregate_test() {
        let order = Order {
            id: "1234".to_string(),
            status: State::Delivered,
            payment_type: Some(PaymentType::Visa),
            amount: 345,
            delivery_type: Some(DeliveryType::Gls),
            items: vec!["1234".to_string(), "2345".to_string()],
            address: Some(Address { street: "Karisevej", house_number: 43, zip: 4690, country: CountryCode::Dk }),
            customer: Some("765432".to_string()),
            action: Action::None,
        };
        let events = add_event(OrderEvent::OrderDelivered { order_id: "1234".to_string(), time: 8 }, store_event_dummy);
        let mut machine = StateMachine::new(State::iter().collect(), OrderEventDiscriminants::iter().collect(), TRANSITIONS.to_owned());
        assert_eq!(aggregate_order(events, Order::new("1234".to_string()), &mut machine), order);
    }

    #[test]
    fn aggregate_test_no_delivery_address() {
        let order = Order {
            id: "1234".to_string(),
            status: State::Delivered,
            payment_type: Some(PaymentType::Visa),
            amount: 345,
            delivery_type: Some(DeliveryType::Gls),
            items: vec!["1234".to_string(), "2345".to_string()],
            address: Some(Address { street: "Taagevej", house_number: 43, zip: 4600, country: CountryCode::Dk }),
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
                address: Address { street: "Taagevej", house_number: 43, zip: 4600, country: CountryCode::Dk },
                time: 0,
            },
            OrderEvent::OrderDetailsAdded {
                order_id: "1234".to_string(),
                delivery_type: DeliveryType::Gls,
                delivery_address: None,
                customer: "54321".to_string(),
                time: 5,
            },
            OrderEvent::OrderPayed { order_id: "1234".to_string(), payment_type: PaymentType::Visa, amount: 345, time: 6 },
            OrderEvent::OrderSent { order_id: "1234".to_string(), time: 7 },
            OrderEvent::OrderDelivered { order_id: "1234".to_string(), time: 8 },
        ];
        let mut machine = StateMachine::new(State::iter().collect(), OrderEventDiscriminants::iter().collect(), TRANSITIONS.to_owned());
        assert_eq!(aggregate_order(events, Order::new("1234".to_string()), &mut machine), order);
    }

    #[test]
    fn aggregate_test_fail_delivery() {
        let order = Order {
            id: "1234".to_string(),
            status: State::DeliveryFailed,
            payment_type: Some(PaymentType::Visa),
            amount: 345,
            delivery_type: Some(DeliveryType::Gls),
            items: vec!["1234".to_string(), "2345".to_string()],
            address: Some(Address { street: "Karisevej", house_number: 43, zip: 4690, country: CountryCode::Dk }),
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
                delivery_type: DeliveryType::Gls,
                delivery_address: Some(Address { street: "Karisevej", house_number: 43, zip: 4690, country: CountryCode::Dk }),
                customer: "54321".to_string(),
                time: 5,
            },
            OrderEvent::OrderPayed { order_id: "1234".to_string(), payment_type: PaymentType::Visa, amount: 345, time: 6 },
            OrderEvent::OrderSent { order_id: "1234".to_string(), time: 7 },
            OrderEvent::OrderDeliveryFailed {
                order_id: "1234".to_string(),
                reason: Reason { reason_code: ReasonCode::PackageLost, reason_message: "Package went into the sea".to_string() },
                time: 8,
            },
        ];
        let mut machine = StateMachine::new(State::iter().collect(), OrderEventDiscriminants::iter().collect(), TRANSITIONS.to_owned());
        assert_eq!(aggregate_order(events, Order::new("1234".to_string()), &mut machine), order);
    }
}
