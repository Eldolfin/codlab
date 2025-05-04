{pkgs, ...}: {
  programs.obs-studio = {
    enable = true;
    plugins = [pkgs.obs-studio-plugins.obs-websocket];
  };
  home.packages = with pkgs; [
    obs-cmd
  ];
  home.file.".config/obs-studio" = {
    source = ./obs-studio;
    recursive = true;
  };
}
