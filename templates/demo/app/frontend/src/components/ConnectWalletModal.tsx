"use client";
import { useState } from "react";
import { Button } from "@/components/ui/button";

import {
  LEATHER,
  MAGIC_EDEN,
  OKX,
  OYL,
  ORANGE,
  PHANTOM,
  UNISAT,
  useLaserEyes,
  WalletIcon,
  WIZZ,
  XVERSE,
} from "@omnisat/lasereyes";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

export default function ConnectWallet({ className }: { className?: string }) {
  const { connect, disconnect, isConnecting, address, provider } =
    useLaserEyes();
  const [isOpen, setIsOpen] = useState(false);

  const walletList: (
    | typeof UNISAT
    | typeof MAGIC_EDEN
    | typeof OYL
    | typeof ORANGE
    | typeof PHANTOM
    | typeof LEATHER
    | typeof XVERSE
    | typeof WIZZ
    | typeof OKX
  )[] = [
      UNISAT,
      MAGIC_EDEN,
      OYL,
      ORANGE,
      PHANTOM,
      LEATHER,
      XVERSE,
      WIZZ,
      OKX,
    ];

  const handleConnect = async (
    walletName:
      | typeof UNISAT
      | typeof MAGIC_EDEN
      | typeof OYL
      | typeof ORANGE
      | typeof PHANTOM
      | typeof LEATHER
      | typeof XVERSE
      | typeof WIZZ
      | typeof OKX
  ) => {
    if (provider === walletName) {
      await disconnect();
    } else {
      setIsOpen(false);
      await connect(walletName as never);
    }
  };

  const truncateAddress = (addr: string) => {
    return `${addr.slice(0, 4)}...${addr.slice(-8)}`;
  };

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger className={cn("mb-4 text-arch-orange font-bold py-2 px-4 rounded-lg hover:bg-arch-black transition duration-300", className)}
      >
        {address
          ? truncateAddress(address)
          : isConnecting
            ? "Connecting..."
            : "Connect Wallet"}
      </DialogTrigger>
      <DialogContent
        className={"bg-arch-black border-2 border-arch-gray text-arch-white rounded-lg"}
      >
        <DialogHeader className={"gap-2"}>
          <DialogTitle
            className={"text-center text-white font-bold tracking-wide"}
          >
            Connect Wallet
          </DialogTitle>
          <DialogDescription className="flex flex-col gap-2 w-full">
            {walletList.map(
              (
                walletName:
                  | typeof UNISAT
                  | typeof MAGIC_EDEN
                  | typeof OYL
                  | typeof ORANGE
                  | typeof PHANTOM
                  | typeof LEATHER
                  | typeof XVERSE
                  | typeof WIZZ
                  | typeof OKX
              ) => {
                return (
                  <Button
                    key={walletName}
                    onClick={() => handleConnect(walletName)}
                    className={cn("w-full text-lg  text-arch-white font-bol py-2 px-4 rounded-lg hover:bg-arch-orange hover:text-black transition-all duration-300 bg-black border border-arch-gray")}
                    size="lg"
                  >
                    <WalletIcon size={128} walletName={walletName} />
                    {provider === walletName ? "disconnect " : ""}
                    {walletName.replace("-", " ")}
                  </Button>
                );
              },
            )}
          </DialogDescription>
        </DialogHeader>
      </DialogContent>
    </Dialog>
  );
}

