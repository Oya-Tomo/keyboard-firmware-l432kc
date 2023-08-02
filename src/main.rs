#![no_std]
#![no_main]

use hal::{pwr::PwrExt, rcc::RccExt, timer::Timer, usb};

use panic_halt as _;
use stm32l4xx_hal as hal;

use cortex_m_rt::entry;
use hal::{prelude::*, stm32};
use stm32_usbd::UsbBus;
use usb_device::prelude::{UsbDeviceBuilder, UsbVidPid};
use usbd_hid::{
    descriptor::{KeyboardReport, SerializedDescriptor},
    hid_class::{
        HIDClass, HidClassSettings, HidCountryCode, HidProtocol, HidSubClass, ProtocolModeConfig,
    },
};
use usbd_hid_device::USB_CLASS_HID;

fn enable_crs() {
    let rcc = unsafe { &(*stm32::RCC::ptr()) };
    rcc.apb1enr1.modify(|_, w| w.crsen().set_bit());
    let crs = unsafe { &(*stm32::CRS::ptr()) };
    // Initialize clock recovery
    // Set autotrim enabled.
    crs.cr.modify(|_, w| w.autotrimen().set_bit());
    // Enable CR
    crs.cr.modify(|_, w| w.cen().set_bit());
}

/// Enables VddUSB power supply
fn enable_usb_pwr() {
    // Enable PWR peripheral
    let rcc = unsafe { &(*stm32::RCC::ptr()) };
    rcc.apb1enr1.modify(|_, w| w.pwren().set_bit());

    // Enable VddUSB
    let pwr = unsafe { &*stm32::PWR::ptr() };
    pwr.cr2.modify(|_, w| w.usv().set_bit());
}

#[entry]
fn main() -> ! {
    let dp = hal::pac::Peripherals::take().unwrap();
    // let cp = peripheral::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let mut pwr = dp.PWR.constrain(&mut rcc.apb1r1);
    let clocks = rcc
        .cfgr
        .hsi48(true)
        .sysclk(80.MHz())
        .hclk(48.MHz())
        .pclk1(48.MHz())
        .pclk2(48.MHz())
        .freeze(&mut flash.acr, &mut pwr);

    enable_crs();
    enable_usb_pwr();

    // let mut deley = Delay::new(cp.SYST, clocks);

    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb2);
    let mut pb3 = gpiob
        .pb3
        .into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);

    let pb4 = gpiob
        .pb4
        .into_pull_down_input(&mut gpiob.moder, &mut gpiob.pupdr);
    let pb5 = gpiob
        .pb5
        .into_pull_down_input(&mut gpiob.moder, &mut gpiob.pupdr);

    let mut gpioa = dp.GPIOA.split(&mut rcc.ahb2);

    let usb_perip = usb::Peripheral {
        usb: dp.USB,
        pin_dm: gpioa
            .pa11
            .into_alternate(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh)
            .set_speed(hal::gpio::Speed::VeryHigh),
        pin_dp: gpioa
            .pa12
            .into_alternate(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh)
            .set_speed(hal::gpio::Speed::VeryHigh),
    };

    let usb_bus = UsbBus::new(usb_perip);

    let mut hid = HIDClass::new_with_settings(
        &usb_bus,
        KeyboardReport::desc(),
        10,
        HidClassSettings {
            subclass: HidSubClass::NoSubClass,
            protocol: HidProtocol::Keyboard,
            config: ProtocolModeConfig::ForceReport,
            locale: HidCountryCode::NotSupported,
        },
    );

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x2718, 0x2818))
        .manufacturer("Oya-Tomo")
        .product("Wavier-Keys")
        .serial_number("KEY")
        .device_class(USB_CLASS_HID)
        .build();
    usb_dev.force_reset().ok();

    let mut timer = Timer::tim15(dp.TIM15, 1000.Hz(), clocks, &mut rcc.apb2);
    timer.start(100.Hz());

    pb3.set_high();

    loop {
        timer.wait().ok();

        if pb4.is_high() {
            hid.push_input(&KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [0x0e, 0, 0, 0, 0, 0],
            })
            .ok();
        } else if pb5.is_high() {
            hid.push_input(&KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [0x05, 0, 0, 0, 0, 0],
            })
            .ok();
        } else {
            hid.push_input(&KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [0x00, 0, 0, 0, 0, 0],
            })
            .ok();
        }

        hid.pull_raw_output(&mut [0; 64]).ok();
        usb_dev.poll(&mut [&mut hid]);
    }
}
