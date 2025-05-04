(import ./lib.nix) rec {
  name = "simple";
  testScript = ''
    start_all()
    client1.wait_for_unit("graphical.target")
    client2.wait_for_unit("graphical.target")
    # TODO: record video
    # c.sleep(60)
    # # open terminal
    # c.send_key("meta_l-ret")
    # c.sleep(3)
    # # clear
    # c.send_key("ctrl-l", delay=0.1)
    # # dezoom
    # for _ in range(4): c.send_key("ctrl-shift-minus", delay=0.1)
    # c.send_chars("fastfetch\n")
    # c.sleep(30)
    client1.screenshot("${name}")
    client2.screenshot("${name}")
  '';
}
