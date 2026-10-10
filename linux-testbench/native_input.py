import socket
import struct


class Pointer:
    def __init__(self):
        self.connection = socket.create_connection(("127.0.0.1", 5900), timeout=5)
        version = self.read(12)
        if version != b"RFB 003.008\n":
            raise RuntimeError("Unsupported local VNC protocol: " + repr(version))
        self.connection.sendall(version)
        choices = self.read(self.read(1)[0])
        if 1 not in choices:
            raise RuntimeError("Local VNC requires authentication")
        self.connection.sendall(b"\x01")
        if self.read(4) != b"\x00\x00\x00\x00":
            raise RuntimeError("Local VNC connection was rejected")
        self.connection.sendall(b"\x01")
        header = self.read(24)
        self.width, self.height = struct.unpack("!HH", header[:4])
        self.read(struct.unpack("!I", header[20:])[0])

    def read(self, size):
        result = bytearray()
        while len(result) < size:
            chunk = self.connection.recv(size - len(result))
            if not chunk:
                raise RuntimeError("Local VNC disconnected")
            result.extend(chunk)
        return bytes(result)

    def click(self, x, y):
        if not (0 <= x < self.width and 0 <= y < self.height):
            raise ValueError("Pointer coordinates exceed the test display")
        self.connection.sendall(b"".join(struct.pack("!BBHH", 5, mask, x, y) for mask in [0, 1, 0]))

    def close(self):
        self.connection.close()
