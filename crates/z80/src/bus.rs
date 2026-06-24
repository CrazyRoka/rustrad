pub trait Bus {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);

    fn port_read(&self, port: u16) -> u8;
    fn port_write(&mut self, port: u16, value: u8);
}
