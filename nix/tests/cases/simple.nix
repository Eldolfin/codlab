(import ./lib.nix) rec {
  name = "simple";
  testScript = ''
    file = "/home/alice/test.md"
    clients = [client1, client2]

    start_all()

    for client in clients:
      client.wait_for_x()

    # start helix
    for client in clients:
      client.execute(
          f"su alice -c 'kitty -o font_size=12 --start-as=fullscreen -e hx {file}' >&2 &"
      )

    for client in clients:
      client.sleep(1)

    # type some text
    for client in clients:
      client.send_chars(f"oHello! My name is {client.name}")
      client.send_key("esc")
      client.sleep(1)

    # write file
    for client in clients:
      client.send_chars(":w\n")

    for client in clients:
      client.wait_for_file(file)
      client.copy_from_vm(file, f"{client.name}")

    assert open(f"{client1.name}/test.md").read() == open(f"{client2.name}/test.md").read()

    client1.screenshot("${name}-client1")
    client2.screenshot("${name}-client2")
  '';
}
