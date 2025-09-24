mod stock;
mod query;
mod user;

pub use stock::*;
pub use query::*;
pub use user::*;

use std::collections::HashMap;

pub struct Market {
    stocks: HashMap<Symbol, Stock>,
    users: HashMap<usize, User>
}

impl Market {
    pub fn new() -> Self {
        Self { stocks: HashMap::new(), users: HashMap::new() }
    }

    pub fn add_stock(&mut self, symbol: Symbol, stock: Stock) {
        self.stocks.insert(symbol, stock);
    }

    pub fn extend_stocks<I>(&mut self, stocks: I)
    where
        I: IntoIterator<Item = (Symbol, Stock)>
    {
        self.stocks.extend(stocks);        
    }

    pub fn resolve(&mut self) -> Vec<(String, Vec<Trade>)> {
        let mut executed_trades = Vec::new();
        
        for stock in self.stocks.values_mut() {
            executed_trades.push((stock.get_name().to_string(), stock.resolve()))
        }

        executed_trades
    }

    pub fn get_stock(&self, symbol: &Symbol) -> Option<&Stock> {
        self.stocks.get(symbol)
    }

    pub fn get_stock_mut(&mut self, symbol: &Symbol) -> Option<&mut Stock> {
        self.stocks.get_mut(symbol)
    }
}
