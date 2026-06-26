pub enum OrderEvent {
    Created(OrderCreated),
    Fulfilled(OrderFulfilled),
}

pub struct OrderCreated {
    // TODO: order_id + give/take amounts, mints, timestamp
}

pub struct OrderFulfilled {
    // TODO: order_id + taker, timestamp
}
