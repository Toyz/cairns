# Formula for a homebrew tap (Toyz/homebrew-tap as Formula/cairns.rb).
#
# The url and sha256 of each bottle come from a tagged release - see
# `SHA256SUMS` on the release page. Bump VERSION and the four digests.
class Cairns < Formula
  desc "A worklog kept as numbered markdown entries: write them, check them, publish them"
  homepage "https://github.com/Toyz/cairns"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/Toyz/cairns/releases/download/v#{version}/cairns-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_DIGEST"
    end
    on_intel do
      url "https://github.com/Toyz/cairns/releases/download/v#{version}/cairns-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_DIGEST"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/Toyz/cairns/releases/download/v#{version}/cairns-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_DIGEST"
    end
  end

  def install
    bin.install "cairns"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/cairns --version")
  end
end
