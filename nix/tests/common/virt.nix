{
  virtualisation = {
    qemu = {
      options = [
        "-display gtk,zoom-to-fit=on,grab-on-hover=on"
      ];
    };
    memorySize = 4096;
    diskSize = 8192;
    cores = 6;
    resolution = {
      x = 1920;
      y = 1080;
    };
  };
}
