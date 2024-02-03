import socket
import struct

SERVER = "play.lbsg.net"

client = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)

data = b"\xFE\xFD\x09" + struct.pack(">l", 9999)
print(data)

client.sendto(data, (SERVER, 19132))
data, address = client.recvfrom(1024)
print(data, address)
if data:
    print("Response received from:", address)

print(data)
print(len(data))
