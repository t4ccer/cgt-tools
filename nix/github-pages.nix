{
  hercules-ci.github-pages.branch = "main";
  perSystem = {pkgs, ...}: {
    hercules-ci.github-pages.settings.contents = ../website;
  };
}
