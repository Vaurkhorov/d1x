use super::{Order, Trade};
use tokio::sync::mpsc;

/// A query to the market.
pub enum Query {
    /// Post a buy order for the stock.
    Buy(String, Order),
    /// Post a sell order for the stock.
    Sell(String, Order),
    /// Query the OHLC prices for the stock.
    Ohlc(String),
    /// Query the pending buy orders for the stock.
    BuyOrders(String),
    /// Query the pending sell orders for the stock.
    SellOrders(String),
    /// New connection
    Connect(mpsc::Sender<QueryResponse>),
}

impl Query {
    pub fn from_json(json: &str, id: usize) -> Option<Self> {
        let query: serde_json::Value = match serde_json::from_str(json) {
            Ok(q) => q,
            Err(e) => {
                println!("Error parsing JSON: {}", e);
                return None;
            },
        };
        let query_type = query["type"].as_str()?;
        let symbol = query["symbol"].as_str();
        println!("symbol: {:#?}", symbol);
        let price = query["price"].as_f64();
        let quantity = query["quantity"].as_u64();

        match query_type {
            "buy" => Some(Query::Buy(symbol?.to_string(), Order::new(id, price?, quantity? as usize))),
            "sell" => Some(Query::Sell(symbol?.to_string(), Order::new(id, price?, quantity? as usize))),
            "ohlc" => Some(Query::Ohlc(symbol?.to_string())),
            "buy_orders" => Some(Query::BuyOrders(symbol?.to_string())),
            "sell_orders" => Some(Query::SellOrders(symbol?.to_string())),
            _ => None,
        }
    }
}

/// A response from the market to a query.
pub enum QueryResponse {
    // Successes
    /// Socket tx stored.
    Connected,
    /// The order was successfully posted.
    OrderPosted,
    /// A vector of pending orders for the stock.
    ///
    /// It contains a limited number of unique prices and their quantities. The number of unique prices is defined by `NO_OF_PRICES_QUERIED`.
    QueriedOrders(Vec<(f64, usize)>),
    /// The open, high, low, close prices for the stock.
    Ohlc(Option<f64>, Option<f64>, Option<f64>, Option<f64>),
    /// Receipt of a completed trade.
    ExecutedTrade(Trade),

    // Errors
    /// The symbol provided was not found.
    SymbolNotFound,
}

impl QueryResponse {
    pub fn to_json(&self) -> String {
        match self {
            QueryResponse::Connected => r#"{"response": "connected"}"#.to_string(),
            QueryResponse::OrderPosted => r#"{"response": "order_posted"}"#.to_string(),
            QueryResponse::QueriedOrders(orders) => {
                let orders: Vec<String> = orders
                    .iter()
                    .map(|(price, quantity)| {
                        format!(
                            r#"{{"price": {:.2}, "quantity": {}}}"#,
                            price, quantity
                        )
                    })
                    .collect();
                format!(r#"{{"response": "queried_orders", "orders": [{}]}}"#, orders.join(","))
            }
            QueryResponse::Ohlc(open, high, low, close) => {
                format!(
                    r#"{{"response": "ohlc", "open": {:?}, "high": {:?}, "low": {:?}, "close": {:?}}}"#,
                    open, high, low, close
                )
            }
            QueryResponse::ExecutedTrade(trade) => {
                format!(
                    r#"{{"response": "executed_trade", "buyer_id": {}, "seller_id": {}, "price": {:.2}, "quantity": {}}}"#,
                    trade.buyer_id, trade.seller_id, trade.price, trade.quantity
                )
            }
            QueryResponse::SymbolNotFound => r#"{"response": "symbol_not_found"}"#.to_string(),
        }
    }
}

