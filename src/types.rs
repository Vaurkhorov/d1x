use chrono::{DateTime, Utc};

// 10 raised to the number of decimals to keep for prices.
const PRICE_PRECISION_FACTOR: f64 = 1e2;

pub struct Stock {
    symbol: String,
    name: String,
    buy_orders: Vec<Order>,
    sell_orders: Vec<Order>,
    ohlc: OHLC,
}

impl Stock {
    pub fn new(symbol: &str, name: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            name: name.to_string(),
            buy_orders: Vec::new(),
            sell_orders: Vec::new(),
            ohlc: OHLC::new(),
        }
    }

    pub fn get_symbol(&self) -> &str {
        &self.symbol
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn add_buy_order(&mut self, order: Order) {
        self.buy_orders.push(order);
        self.sort_orders();
    }

    pub fn add_sell_order(&mut self, order: Order) {
        self.sell_orders.push(order);
        self.sort_orders();
    }

    pub fn resolve(&mut self) -> Vec<Trade> {
        let mut trades = Vec::new();

        for buy_order in &mut self.buy_orders {
            if let Some(lowest_sell_offer) = self.sell_orders.first() {
                if buy_order.get_price() < lowest_sell_offer.get_price() {
                    // Highest buy bid is less than lowest sell offer
                    break;
                }
            } else {
                // No sell orders left
                break;
            }

            for sell_order in &mut self.sell_orders {
                if sell_order.get_quantity() == 0 {
                    // These might be left over after being resolved.
                    continue;
                }

                if buy_order.get_price() >= sell_order.get_price() {
                    let price = if sell_order.get_time() < buy_order.get_time() {
                        sell_order.get_price()
                    } else {
                        buy_order.get_price()
                    };
                    let quantity = buy_order.get_quantity().min(sell_order.get_quantity());

                    buy_order.resolve(quantity);
                    sell_order.resolve(quantity);
                    trades.push(Trade::new(buy_order.creator_id, sell_order.creator_id, price, quantity));
                    self.ohlc.update(price);

                    if buy_order.get_quantity() == 0 {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        self.buy_orders.retain(|order| order.get_quantity() > 0);
        self.sell_orders.retain(|order| order.get_quantity() > 0);

        trades
    }

    fn sort_orders(&mut self) {
        self.buy_orders.sort_by(|a, b| b.price.partial_cmp(&a.price).expect("prices are f64s and should be comparable."));
        self.sell_orders.sort_by(|a, b| a.price.partial_cmp(&b.price).expect("prices are f64s and should be comparable."));
    }

    pub fn get_ohlc(&self) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
        self.ohlc.get()
    }
}

pub struct Order {
    creator_id: usize,
    price: f64,
    quantity: usize,
    time: DateTime<Utc>,
}

impl Order {
    pub fn new(creator_id: usize, price: f64, quantity: usize) -> Self {
        let price = (price * PRICE_PRECISION_FACTOR).trunc() / PRICE_PRECISION_FACTOR;

        Self {
            creator_id,
            price,
            quantity,
            time: Utc::now(),
        }
    }

    pub fn get_value(&self) -> f64 {
        self.price * self.quantity as f64
    }

    pub fn get_price(&self) -> f64 {
        self.price
    }

    pub fn get_quantity(&self) -> usize {
        self.quantity
    }

    pub fn get_time(&self) -> DateTime<Utc> {
        self.time
    }

    pub fn resolve(&mut self, quantity: usize) {
        self.quantity -= quantity;
    }
}

#[derive(Debug)]
pub struct Trade {
    buyer_id: usize,
    seller_id: usize,
    price: f64,
    quantity: usize,
}

impl Trade {
    fn new(buyer_id: usize, seller_id: usize, price: f64, quantity: usize) -> Self {
        Self {
            buyer_id,
            seller_id,
            price,
            quantity,
        }
    }
}

pub struct OHLC {
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
}

impl OHLC {
    fn new() -> Self {
        Self {
            open: None,
            high: None,
            low: None,
            close: None,
        }
    }

    fn update(&mut self, price: f64) {
        if self.open.is_none() {
            self.open = Some(price);
        }

        self.high = Some(self.high.unwrap_or(price).max(price));
        self.low = Some(self.low.unwrap_or(price).min(price));
        self.close = Some(price);
    }

    pub fn get(&self) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
        (self.open, self.high, self.low, self.close)
    }
}