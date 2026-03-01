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
import { useDeleteAlertRule } from "@/hooks/use-alert-rules";
import * as toast from "@/lib/toast";

interface DeleteAlertRuleDialogProps {
  ruleId: string;
  ruleName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function DeleteAlertRuleDialog({
  ruleId,
  ruleName,
  open,
  onOpenChange,
}: DeleteAlertRuleDialogProps) {
  const deleteRule = useDeleteAlertRule();

  function handleConfirm() {
    deleteRule.mutate(ruleId, {
      onSuccess: () => {
        toast.success("Alert rule deleted");
        onOpenChange(false);
      },
      onError: () => {
        toast.error("Failed to delete alert rule");
      },
    });
  }

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete "{ruleName}"?</AlertDialogTitle>
          <AlertDialogDescription>
            This will permanently delete this alert rule. This action cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={handleConfirm}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            {deleteRule.isPending ? "Deleting..." : "Delete"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
