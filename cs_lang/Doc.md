# Automated CI/CD Pipeline for .NET on Azure

This document contains the standard `Dockerfile` for your .NET backend and the GitHub Actions workflow required to automatically build and deploy your container to Azure App Service every time you push to the `main` branch.

## 1. Dockerfile
Place this file in the root of your repository (next to your `.sln` file).

```dockerfile
# Stage 1: Build the application
FROM mcr.microsoft.com/dotnet/sdk:8.0 AS build
WORKDIR /src

# Copy the project file and restore dependencies (Replace 'YourProjectName' with your actual project name)
COPY ["YourProjectName/YourProjectName.csproj", "YourProjectName/"]
RUN dotnet restore "YourProjectName/YourProjectName.csproj"

# Copy the remaining source code and build
COPY . .
WORKDIR "/src/YourProjectName"
RUN dotnet publish "YourProjectName.csproj" -c Release -o /app/publish /p:UseAppHost=false

# Stage 2: Create the runtime image
FROM mcr.microsoft.com/dotnet/aspnet:8.0 AS final
WORKDIR /app
EXPOSE 8080

# Copy the published output from the build stage
COPY --from=build /app/publish .
ENTRYPOINT ["dotnet", "YourProjectName.dll"]
```

## 2. GitHub Actions Workflow
Create a new file in your repository at this exact path: `.github/workflows/deploy.yml` and paste the following code into it.

```yaml
name: Build and Deploy .NET Container to Azure

on:
  push:
    branches:
      - main # Triggers the workflow when you push to the main branch

env:
  ACR_LOGIN_SERVER: myuniqueacrname123.azurecr.io # Replace with your ACR login server
  ACR_NAME: myuniqueacrname123                    # Replace with your ACR name
  IMAGE_NAME: mydotnetbackend                     # Name of your docker image
  WEBAPP_NAME: mydotnetwebapp123                  # Replace with your Azure Web App name

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest

    steps:
      # 1. Check out the repository code
      - name: Checkout Code
        uses: actions/checkout@v4

      # 2. Authenticate with Azure using GitHub Secrets
      - name: Log in to Azure
        uses: azure/login@v2
        with:
          creds: ${{ secrets.AZURE_CREDENTIALS }}

      # 3. Authenticate with your Azure Container Registry (ACR)
      - name: Log in to ACR
        run: az acr login --name ${{ env.ACR_NAME }}

      # 4. Build the Docker image and tag it with the unique GitHub commit hash
      - name: Build Docker Image
        run: |
          docker build -t ${{ env.ACR_LOGIN_SERVER }}/${{ env.IMAGE_NAME }}:${{ github.sha }} .
          docker tag ${{ env.ACR_LOGIN_SERVER }}/${{ env.IMAGE_NAME }}:${{ github.sha }} ${{ env.ACR_LOGIN_SERVER }}/${{ env.IMAGE_NAME }}:latest

      # 5. Push the built image to ACR
      - name: Push Image to ACR
        run: |
          docker push ${{ env.ACR_LOGIN_SERVER }}/${{ env.IMAGE_NAME }}:${{ github.sha }}
          docker push ${{ env.ACR_LOGIN_SERVER }}/${{ env.IMAGE_NAME }}:latest

      # 6. Deploy the new image to Azure App Service
      - name: Deploy to Azure Web App
        uses: azure/webapps-deploy@v3
        with:
          app-name: ${{ env.WEBAPP_NAME }}
          images: ${{ env.ACR_LOGIN_SERVER }}/${{ env.IMAGE_NAME }}:${{ github.sha }}
```

## 3. Required GitHub Secrets Setup
For the pipeline to securely access your Azure environment, you must configure an `AZURE_CREDENTIALS` secret in your GitHub repository.

1. Open your terminal and run this Azure CLI command (replace `<subscription-id>` and `<resource-group-name>`):
   ```bash
   az ad sp create-for-rbac --name "myAppDeploySp" --role contributor --scopes /subscriptions/<subscription-id>/resourceGroups/<resource-group-name> --json-auth
   ```
2. The command will output a JSON object. Copy the entire JSON block.
3. Go to your GitHub repository -> **Settings** -> **Secrets and variables** -> **Actions** -> **New repository secret**.
4. Name the secret `AZURE_CREDENTIALS` and paste the JSON block into the value field, then save.
