import { PixiNightCatAvatar } from "./PixiNightCatAvatar";

interface PlaceholderPetProps {
  detail?: string;
}

export function PlaceholderPet({ detail }: PlaceholderPetProps) {
  return <PixiNightCatAvatar detail={detail} />;
}
