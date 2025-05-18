import can

filters = [
    {"can_id": 0x123, "can_mask": 0x7FF, "extended": False},
]

with can.Bus(
    interface="seeedstudio",
    channel="/dev/ttyUSB0",
    bitrate=125000,
    can_filters=filters,
    receive_own_messages=False
) as bus:
    # iterate over received messages
    for msg in bus:
        print(f"{msg.arbitration_id:X}: {msg.data}")

        temperature = int.from_bytes(msg.data[0:2], "big", signed=True) / 100.0
        humidity = int.from_bytes(msg.data[2:4], "big", signed=True) / 100.0

        print(f"temperature: {temperature:2}°C, humidity: {humidity:2}%RH")
