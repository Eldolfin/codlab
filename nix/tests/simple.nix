(import ./lib.nix) {
  name = "simple";
  testScript = ''
    txt_file = "/home/alice/test.md"
    clients = [client1, client2]
    recordings = []

    start_all()

    for client in clients:
      client.wait_for_x()

    for client in clients:
      client.execute("su alice -c 'obs' >/dev/null >&2 &")

    for client in clients:
      client.sleep(5)

    for client in clients:
      client.execute("su alice -c 'obs-cmd recording start'")
    # start helix
    for client in clients:
      client.execute(
          f"su alice -c 'kitty -o font_size=12 --start-as=fullscreen -e hx {txt_file}' >/dev/null >&2 &"
      )

    for client in clients:
      client.sleep(3)

    # type some text
    for client in clients:
      client.send_chars(f"oHello! My name is {client.name}")
      client.send_key("esc")
      client.sleep(3)

    for client in clients:
      # write file
      client.send_chars(":w\n")
      # stop screen recording
      recordings.append(
        client.execute("su alice -c 'obs-cmd recording stop'")[1].split('"')[1]
      )

    for (client, recording) in zip(clients, recordings):
      client.wait_for_file(txt_file)
      client.copy_from_vm(txt_file, f"{client.name}")
      client.copy_from_vm(recording, f"{client.name}")

    assert open(f"{client1.name}/test.md").read() == open(f"{client2.name}/test.md").read()
  '';
}
