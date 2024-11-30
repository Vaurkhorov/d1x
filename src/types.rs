use std::collections::HashMap;
use chrono::{DateTime, Utc};

// 10 raised to the number of decimals to keep for prices.
const PRICE_PRECISION_FACTOR: f64 = 1e2;
/// Number of unique prices that are checked for in the order book.
const NO_OF_PRICES_QUERIED: usize = 5;

/// Holds details for a stock and its orders.
pub struct Stock {
    /// The symbol of the stock (e.g., "ORT").
    /// 
    /// This is used for querying the stock, and so must be unique.
    symbol: String,
    /// The full name of the stock (e.g., "Orchard de Rosa et Tulipan")
    name: String,
    /// Buy orders for the stock.
    buy_orders: Vec<Order>,
    /// Sell orders for the stock.
    sell_orders: Vec<Order>,
    /// Open, high, low, close prices for the stock.
    ohlc: OHLC,
}

impl Stock {
    /// Creates a new stock with the given symbol and name.
    pub fn new(symbol: &str, name: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            name: name.to_string(),
            buy_orders: Vec::new(),
            sell_orders: Vec::new(),
            ohlc: OHLC::new(),
        }
    }

    /// Returns the symbol of the stock.
    pub fn get_symbol(&self) -> &str {
        &self.symbol
    }

    /// Returns the name of the stock.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Adds a buy order to the stock.
    pub fn add_buy_order(&mut self, order: Order) {
        self.buy_orders.push(order);
        self.sort_orders();
    }

    /// Adds a sell order to the stock.
    pub fn add_sell_order(&mut self, order: Order) {
        self.sell_orders.push(order);
        self.sort_orders();
    }

    /// Returns pending buy orders for the stock.
    pub fn get_buy_orders(&self) -> Vec<(f64, usize)> {
        let mut pricelist = HashMap::<usize, usize>::new();

        for order in &self.buy_orders {
            let price = order.get_unadjusted_price();
            let quantity = order.get_quantity();

            if let Some(existing_price) = pricelist.get(&price) {
                pricelist.insert(price, existing_price + quantity);
            } else {
                if pricelist.len() >= NO_OF_PRICES_QUERIED {
                    break;
                }
                pricelist.insert(price, quantity);
            }
        }

        pricelist.iter().map(|(price, quantity)| ((*price as f64) / PRICE_PRECISION_FACTOR, *quantity)).collect()
    }

    /// Returns pending sell orders for the stock.
    pub fn get_sell_orders(&self) -> Vec<(f64, usize)> {
        let mut pricelist = HashMap::<usize, usize>::new();

        for order in &self.sell_orders {
            let price = order.get_unadjusted_price();
            let quantity = order.get_quantity();

            if let Some(existing_price) = pricelist.get(&price) {
                pricelist.insert(price, existing_price + quantity);
            } else {
                if pricelist.len() >= NO_OF_PRICES_QUERIED {
                    break;
                }
                pricelist.insert(price, quantity);
            }
        }

        pricelist.iter().map(|(price, quantity)| ((*price as f64) / PRICE_PRECISION_FACTOR, *quantity)).collect()
    }

    /// Resolves trades between buy and sell orders.
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

    /// Sorts buy and sell orders by price.
    fn sort_orders(&mut self) {
        self.buy_orders.sort_by(|a, b| b.price.partial_cmp(&a.price).expect("prices are f64s and should be comparable."));
        self.sell_orders.sort_by(|a, b| a.price.partial_cmp(&b.price).expect("prices are f64s and should be comparable."));
    }

    /// Returns the open, high, low, close prices for the stock.
    pub fn get_ohlc(&self) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
        self.ohlc.get()
    }
}

/// An order to buy or sell a stock.
pub struct Order {
    /// The ID of the creator of the order.
    creator_id: usize,
    /// The price per stock.
    price: usize,
    /// The quantity of the order.
    quantity: usize,
    /// The time the order was created.
    /// 
    /// The price listed on the order that was created earlier is considered while resolving orders.
    time: DateTime<Utc>,
}

impl Order {
    /// Creates a new order with the given creator ID, price, and quantity.
    pub fn new(creator_id: usize, price: f64, quantity: usize) -> Self {
        let price = (price * PRICE_PRECISION_FACTOR) as usize;

        Self {
            creator_id,
            price,
            quantity,
            time: Utc::now(),
        }
    }

    /// Returns the total value of the order.
    pub fn get_value(&self) -> f64 {
        (self.price as f64) * (self.quantity as f64) / PRICE_PRECISION_FACTOR
    }

    /// Returns the price per stock of the order.
    pub fn get_price(&self) -> f64 {
        self.price as f64 / PRICE_PRECISION_FACTOR
    }

    /// Returns the price per stock WITHOUT adjusting for the precision factor.
    fn get_unadjusted_price(&self) -> usize {
        self.price
    }

    /// Returns the quantity of the order.
    pub fn get_quantity(&self) -> usize {
        self.quantity
    }

    /// Returns the time the order was created.
    pub fn get_time(&self) -> DateTime<Utc> {
        self.time
    }

    /// Reduces the quantity of the order by the given amount.
    pub fn resolve(&mut self, quantity: usize) {
        self.quantity -= quantity;
    }
}

#[derive(Debug)]
/// A log of a resolved trade between a buyer and a seller.
pub struct Trade {
    /// The ID of the buyer.
    buyer_id: usize,
    /// The ID of the seller.
    seller_id: usize,
    /// The price per stock.
    price: f64,
    /// The quantity of the trade.
    quantity: usize,
}

impl Trade {
    /// Creates a new trade with the given buyer ID, seller ID, price, and quantity.
    fn new(buyer_id: usize, seller_id: usize, price: f64, quantity: usize) -> Self {
        Self {
            buyer_id,
            seller_id,
            price,
            quantity,
        }
    }
}

/// Open, high, low, close prices for a stock.
pub struct OHLC {
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
}

impl OHLC {
    /// Creates a blank OHLC struct.
    /// 
    /// Values are all set to `None` until the first update.
    fn new() -> Self {
        Self {
            open: None,
            high: None,
            low: None,
            close: None,
        }
    }

    /// Updates the OHLC values according to the latest trade price provided.
    fn update(&mut self, latest_price: f64) {
        if self.open.is_none() {
            self.open = Some(latest_price);
        }

        self.high = Some(self.high.unwrap_or(latest_price).max(latest_price));
        self.low = Some(self.low.unwrap_or(latest_price).min(latest_price));
        self.close = Some(latest_price);
    }

    /// Returns the open, high, low, close prices.
    pub fn get(&self) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
        (self.open, self.high, self.low, self.close)
    }
}

/// A query to the market.
pub enum Query {
    /// Post a buy order for the stock.
    Buy(String, Order),
    /// Post a sell order for the stock.
    Sell(String, Order),
    /// Query the OHLC prices for the stock.
    OHLC(String),
    /// Query the pending buy orders for the stock.
    BuyOrders(String),
    /// Query the pending sell orders for the stock.
    SellOrders(String),
}

/// A response from the market to a query.
pub enum QueryResponse {
    // Successes
    /// The order was successfully posted.
    OrderPosted,
    /// A vector of pending orders for the stock.
    /// 
    /// It contains a limited number of unique prices and their quantities. The number of unique prices is defined by `NO_OF_PRICES_QUERIED`.
    QueriedOrders(Vec<(f64, usize)>),
    /// The open, high, low, close prices for the stock.
    OHLC(Option<f64>, Option<f64>, Option<f64>, Option<f64>),
    
    // Errors
    /// The symbol provided was not found.
    SymbolNotFound,
}