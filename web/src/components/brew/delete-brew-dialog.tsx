import { useNavigate } from "react-router-dom";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { useDeleteBrew } from "@/hooks/use-brews";
import * as toast from "@/lib/toast";

interface DeleteBrewDialogProps {
  brewId: string;
  brewName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function DeleteBrewDialog({
  brewId,
  brewName,
  open,
  onOpenChange,
}: DeleteBrewDialogProps) {
  const navigate = useNavigate();
  const deleteBrew = useDeleteBrew();

  function handleConfirm() {
    deleteBrew.mutate(brewId, {
      onSuccess: () => {
        toast.success("Brew deleted");
        navigate("/brews");
      },
      onError: () => {
        toast.error("Failed to delete brew");
      },
    });
  }

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete "{brewName}"?</AlertDialogTitle>
          <AlertDialogDescription>
            This action cannot be undone. This will permanently delete the brew
            and all associated data.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={handleConfirm}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            {deleteBrew.isPending ? "Deleting..." : "Delete"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
