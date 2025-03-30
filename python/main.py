from gnuradio import gr, blocks, network
import osmosdr

class SDRStreamer(gr.top_block):
    def __init__(self):
        gr.top_block.__init__(self)

        host = "192.168.X.X"
        port = 4001

        self.src = osmosdr.source(args="numchan=1 driver=sdrplay")
        self.src.set_sample_rate(2e6)
        self.src.set_center_freq(915e6)
        self.src.set_gain(40)

        self.udp_sink = network.udp_sink(gr.sizeof_gr_complex, 1, host, port, 1472)
        self.connect(self.src, self.udp_sink)

if __name__ == '__main__':
    tb = SDRStreamer()
    tb.run()
