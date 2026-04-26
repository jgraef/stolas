# STOLAS - Simple Telescope for Observation of Large Astronomical Sources

![Photo of the telescope: A stove-pipe, one side covered with aluminium foil, zip-tied to a piece of wood and a tripod. Some electronics are also zip-tied to the tripod. It is standing outside, pointing into the sky.](docs/20260425_142016.jpg)

This is our first attempt at measuring the emissions of neutral hydrogen at 21cm / 1420 MHz.

## Construction

The antenna is an open waveguide, often called a "cantenna" or "stove pipe antenna". It's made from an old piece of stove pipe. Dimensions can be calculated [here][1]. Astropeiler Stockert also has a [fantastic analysis][2] of simple DIY antennas for hydrogen observation. We drilled a small hole for a SMA connector that has the probe (1mm copper wire) soldered to the center pin. But we recommend using a bigger socket that can be screwed in, because we weren't able to solder to the stove pipe. For now the SMA connector is held in place by friction, but electrical connection to the stove pipe was checked.

The antenna is then connected to a [Nooelec LaNA][3] to amplify the signal and a [1420 MHz bandpass filter][4] to avoid unwanted signals. We would have used the [Nooelec SAWbird+ H1][5] instead, but it was unavailable at the time. We tried verifying the setup with a cheap [NanoVNA clone][9], but were unable to get it to work at 1420 MHz. The seller states it works up to 1.5 GHz, but it might not go that high (which is fine for such a cheap clone); or we haven't calibrated it correctly. In the future we would also like to build our own bandpass filter, so that we can tune it easily.

Finally the signal goes into a [RTLSDR Blog v4][6]. It's a really good SDR for that price-range and can be used for a variety of projects. This SDR works well upto a sampling rate of 2.4 MHz, so we can sample a spectrum of ±1.2 MHz around the center frequency of 1420 MHz. It can go higher, but might drop some samples - which might be fine, but needs testing.

The RTL-SDR is connected to a [Raspberry Pi 4][8] that runs software to capture and preprocess the signal. The Raspberry Pi 4 seems to be ideal, because it has WiFi, so we don't have to run an Ethernet cable to access the data, and it can handle the preprocessing easily.

## Software

Unfortunately the hydrogen emissions from cosmic sources are really weak, so you will not be able to see anything in SDR++ or a similar software. [This][7] presentation contains a good overview of what kind of processing needs to be done.

Also without any processing before storing the data you'd need to store at least ~8 GiB/h.

So it's a good idea to do some initial processing right after sampling. We split the sample stream into chunks of 512 samples and perform an FFT on it. The samples themself and the output of the FFT will be complex numbers, representing a random phase and the amplitude of each frequency component. We take the magnitude to get rid of the phase.

At this point we would get 512 32-bit floats for every 512 samples, representing a spectrum with 512 frequency bins, each 4.6875 kHz wide (2.4 MHz / 512). But we still have a lot of noise. Fortunately this noise is mostly gaussian, meaning we can remove it by taking an average. We take an average over 50000 spectra. The previously mentioned presentation used this average size, but they do another average (10x) later.

After this initial processing we will get a spectrum every ~10.6s (512 * 50000 / 2.4 MHz). Each spectrum consists of 512 32-bit floats, thus we will only have to store about 675 kiB/h.

The preprocessed signal is then stored on the Raspberry Pi's SD card, and can be streamed over the network.

TODO: Web interface

## User Interface

### TODO

- Configure parameters and meta-data (that is put into the recording file)
- Start/Stop measurement
- Show status: measurement, system time, system temperature, location, pointing, beam coverage
- Tool to get location/pointing config via mobile phone (accelerometer/compass)

## Calibration

### TODO

- Take measurement without antenna connected.
- Take measurement of cold sky
- Take measurement of sun (for estimation of beam width)

## Post-processing

TODO

## Results

TODO

# Improvements & Lessons learned

- Use SMA or N connector that can be screwed in.
- Don't use a black stove-pipe. It'll heat up quickly in the sun, degrading the signal.
- The Raspberry Pi 4 doesn't have a real-time clock. We must wait for NTP to synchronize the time before starting a measurement. A RTC module such as the [DS3231 AT24C32][11] might be a good idea.

# TODO

- Sensors:
  - [Compass][12] and [Accelerometer][13] to measure elevation and azimuth. Might need extra wires to have the sensors aligned with the antenna.
  - [Real-time clock][11] to keep time when NTP is not available.
  - [`embedded-hal`][10] for using I2C or SPI in Rust.
- Measure system temperature.
- Use FITS for storing measurements. [Recommendations][14], [fitsrs][15]

[1]: https://3g-aerial.biz/en/online-calculations/antenna-calculations/cantenna-online-calculator
[2]: https://www.astropeiler.de/en/beobachtungen-der-21-cm-linie-mit-einfachen-mitteln/
[3]: https://www.nooelec.com/store/lana.html
[4]: https://www.amazon.de/-/en/dp/B0FTR9LJDT?ref=ppx_yo2ov_dt_b_fed_asin_title
[5]: https://www.nooelec.com/store/sawbird-h1.html
[6]: https://www.rtl-sdr.com/V4/
[7]: https://www.youtube.com/live/vHxzKCaay0w?si=ypYq407gME_b95hm&t=1983
[8]: https://www.raspberrypi.com/products/raspberry-pi-4-model-b/
[9]: https://www.amazon.de/-/en/dp/B0B93FNW27
[10]: https://docs.rs/embedded-hal-async/latest/embedded_hal_async/i2c/index.html
[11]: https://www.amazon.de/-/en/gp/product/B0CZDFTJV5
[12]: https://www.amazon.de/-/en/gp/product/B0CH585MKV
[13]: https://www.amazon.de/-/en/gp/product/B0CW4NNWZP?smid=A11EG882P3SP30
[14]: https://heasarc.gsfc.nasa.gov/docs/heasarc/ofwg/ofwg_recomm.html
[15]: https://docs.rs/fitrs/latest/fitrs/index.html
