{
  virtualisation = {
    qemu = {
      options = [
        "-display gtk,zoom-to-fit=on,grab-on-hover=on"
      ];
    };
    memorySize = 4096;
    diskSize = 8192;
    cores = 4;
    resolution = {
      x = 1280;
      y = 800;
    };
  };
}
