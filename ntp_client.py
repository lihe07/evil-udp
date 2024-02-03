import socket

NTP_SERVER = "222.175.125.93"


def sntp_client():
    client = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    data = ("\x1b" + 47 * "\0").encode()
    data = ("\x17\x00\x03\x2a" + 61 * "\0").encode()  # (monlist)

    print(data)

    client.sendto(data, (NTP_SERVER, 123))
    data, address = client.recvfrom(1024)
    if data:
        print("Response received from:", address)

    print(data)
    print(len(data))


if __name__ == "__main__":
    sntp_client()
